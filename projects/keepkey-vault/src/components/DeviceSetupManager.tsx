import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useCommonDialogs } from '../hooks/useCommonDialogs';

interface DeviceSetupManagerProps {
  // This component runs in the background and manages setup requirements
}

interface IncompleteDevice {
  device_id: string;
  serial_number?: string;
  setup_step_completed: number;
  setup_started_at?: number;
}

export function DeviceSetupManager({}: DeviceSetupManagerProps) {
  const [managedDevices, setManagedDevices] = useState<Set<string>>(new Set());
  const { showSetup } = useCommonDialogs();

  useEffect(() => {
    let isMounted = true;

    console.log('🔧 DeviceSetupManager: Setting up listeners...');

    // Check for incomplete devices on app startup
    const checkIncompleteDevices = async () => {
      try {
        const incompleteDevices = await invoke<IncompleteDevice[]>('get_incomplete_setup_devices');
        
        if (incompleteDevices.length > 0 && isMounted) {
          console.log(`📋 Found ${incompleteDevices.length} device(s) with incomplete setup:`, incompleteDevices);
          
          // Launch setup for the most recently started device
          const mostRecent = incompleteDevices.sort((a, b) => 
            (b.setup_started_at || 0) - (a.setup_started_at || 0)
          )[0];
          
          if (mostRecent && !managedDevices.has(mostRecent.device_id)) {
            console.log(`🚀 Auto-launching setup for device: ${mostRecent.device_id} (step ${mostRecent.setup_step_completed})`);
            
            setManagedDevices(prev => new Set([...prev, mostRecent.device_id]));
            showSetup({ 
              initialDeviceId: mostRecent.device_id,
              resumeFromIncomplete: true 
            });
          }
        }
      } catch (error) {
        console.error('Failed to check incomplete devices:', error);
      }
    };

    // Check for devices that need setup (fallback check after frontend_ready)
    const checkDevicesThatNeedSetup = async () => {
      try {
        console.log('🔍 DeviceSetupManager: Checking for devices that need setup...');
        const devicesThatNeedSetup = await invoke<Array<{ device_id: string; device_name: string; serial_number: string }>>('get_devices_needing_setup');
        
        if (devicesThatNeedSetup.length > 0 && isMounted) {
          console.log(`📋 Found ${devicesThatNeedSetup.length} device(s) that need setup:`, devicesThatNeedSetup);
          
          // Launch setup for the first device that needs setup
          const device = devicesThatNeedSetup[0];
          if (!managedDevices.has(device.device_id)) {
            console.log(`🚀 Launching setup for device: ${device.device_id}`);
            
            setManagedDevices(prev => new Set([...prev, device.device_id]));
            showSetup({ 
              initialDeviceId: device.device_id,
              mandatory: true,
              deviceName: device.device_name,
              serialNumber: device.serial_number
            });
          }
        }
      } catch (error) {
        console.error('Failed to check devices that need setup:', error);
      }
    };

    // Listen for setup-required events from newly connected devices
    const setupEventListener = async () => {
      try {
        console.log('🎯 DeviceSetupManager: Setting up device:setup-required listener...');
        const unlisten = await listen('device:setup-required', (event: any) => {
          const { device_id, device_name, serial_number } = event.payload;
          
          console.log(`⚠️ Device setup required for: ${device_id} (${device_name})`);
          
          // Prevent launching multiple setup wizards for the same device
          if (!managedDevices.has(device_id)) {
            setManagedDevices(prev => new Set([...prev, device_id]));
            
            // Show setup wizard with a slight delay to ensure device is fully connected
            setTimeout(() => {
              console.log(`🚀 Launching mandatory setup for device: ${device_id}`);
              showSetup({ 
                initialDeviceId: device_id,
                mandatory: true,
                deviceName: device_name,
                serialNumber: serial_number
              });
            }, 1000);
          } else {
            console.log(`⏭️ Setup already managed for device: ${device_id}`);
          }
        });

        return unlisten;
      } catch (error) {
        console.error('Failed to set up device setup listener:', error);
        return () => {};
      }
    };

    // Listen for setup completion events to remove devices from managed set
    const setupCompleteListener = async () => {
      try {
        const unlisten = await listen('setup:completed', (event: any) => {
          const { device_id } = event.payload;
          console.log(`✅ Setup completed for device: ${device_id}`);
          
          setManagedDevices(prev => {
            const newSet = new Set(prev);
            newSet.delete(device_id);
            return newSet;
          });
        });

        return unlisten;
      } catch (error) {
        console.error('Failed to set up setup completion listener:', error);
        return () => {};
      }
    };

    // Listen for frontend ready events to trigger fallback checks
    const frontendReadyListener = async () => {
      try {
        console.log('🎯 DeviceSetupManager: Setting up frontend-ready listener for fallback checks...');
        const unlisten = await listen('frontend:ready', async () => {
          console.log('🎯 DeviceSetupManager: Frontend ready detected, running fallback setup checks...');
          // Add a small delay to ensure all events have been processed
          setTimeout(async () => {
            await checkDevicesThatNeedSetup();
          }, 500);
        });

        return unlisten;
      } catch (error) {
        console.error('Failed to set up frontend ready listener:', error);
        return () => {};
      }
    };

    // Initialize everything
    const initialize = async () => {
      // Check for incomplete devices first
      await checkIncompleteDevices();
      
      // Set up event listeners IMMEDIATELY
      const unlistenSetupRequired = await setupEventListener();
      const unlistenSetupComplete = await setupCompleteListener();
      const unlistenFrontendReady = await frontendReadyListener();
      
      // Run an immediate fallback check in case events were already sent
      setTimeout(async () => {
        console.log('🔍 DeviceSetupManager: Running immediate fallback check...');
        await checkDevicesThatNeedSetup();
      }, 100);
      
      return () => {
        unlistenSetupRequired();
        unlistenSetupComplete();
        unlistenFrontendReady();
      };
    };

    let cleanup: (() => void) | undefined;
    initialize().then(fn => {
      if (isMounted) {
        cleanup = fn;
      }
    });

    return () => {
      isMounted = false;
      cleanup?.();
    };
  }, [showSetup, managedDevices]);

  // This component doesn't render anything visible
  return null;
}

export default DeviceSetupManager; 