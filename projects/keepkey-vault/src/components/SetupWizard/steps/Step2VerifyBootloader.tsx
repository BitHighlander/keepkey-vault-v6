import {
  Box,
  Button,
  Card,
  Text,
  VStack,
  HStack,
  Icon,
  Spinner,
  Badge,
  Code,
  Image,
} from "@chakra-ui/react";
import { FaShieldAlt, FaCheckCircle, FaExclamationTriangle, FaUsb, FaSync, FaArrowDown, FaCheck } from "react-icons/fa";
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import bootloaderGif from "../../../assets/gif/kk.gif";

interface StepProps {
  onNext: (data?: any) => void;
  onPrevious: () => void;
  onSkip?: () => void;
  deviceId?: string;
  stepData?: any;
}

type FlowState = 'checking' | 'needs_update' | 'instructions' | 'wait_disconnect' | 'disconnected' | 'wait_reconnect' | 'bootloader_ready' | 'updating' | 'complete' | 'error';

interface DeviceFeatures {
  bootloaderVersion?: string;
  version: string;
  initialized: boolean;
  bootloaderMode?: boolean;
}

interface CompletedSteps {
  step1_unplugged: boolean;
  step2_holding: boolean;
  step3_plugged: boolean;
  step4_bootloader: boolean;
}

const LATEST_BOOTLOADER_VERSION = "2.1.4";

export function Step2VerifyBootloader({ onNext, deviceId }: StepProps) {
  const [features, setFeatures] = useState<DeviceFeatures | null>(null);
  const [flowState, setFlowState] = useState<FlowState>('checking');
  const flowStateRef = useRef<FlowState>('checking');
  const [error, setError] = useState<string | null>(null);
  const [disconnectedDeviceId, setDisconnectedDeviceId] = useState<string | null>(null);
  const [completedSteps, setCompletedSteps] = useState<CompletedSteps>({
    step1_unplugged: false,
    step2_holding: false,
    step3_plugged: false,
    step4_bootloader: false,
  });

  // Keep flowStateRef in sync with flowState
  useEffect(() => {
    flowStateRef.current = flowState;
  }, [flowState]);

  useEffect(() => {
    if (deviceId) {
      console.log(`🔧 [BOOTLOADER] Starting bootloader check for device: ${deviceId}`);
      getDeviceFeatures();
      
      // Set up event listeners immediately when component mounts
      console.log('🎧 [BOOTLOADER] Setting up initial event listeners...');
      setupInitialEventListeners().catch(error => {
        console.error('❌ [BOOTLOADER] Failed to set up event listeners:', error);
      });
    }
    
    // Cleanup function
    return () => {
      console.log('🧹 [BOOTLOADER] Component unmounting, cleaning up listeners...');
      if ((window as any).bootloaderListeners) {
        try {
          (window as any).bootloaderListeners.disconnect?.();
          (window as any).bootloaderListeners.connect?.();
          console.log('✅ [BOOTLOADER] Event listeners cleaned up');
        } catch (error) {
          console.error('❌ [BOOTLOADER] Error cleaning up listeners:', error);
        }
      }
    };
  }, [deviceId]);

  // Set up initial event listeners that are always active
  const setupInitialEventListeners = async () => {
    try {
      console.log('🎧 [BOOTLOADER] Setting up disconnect/connect listeners...');
      console.log('🎧 [BOOTLOADER] Our device ID is:', deviceId);
      console.log('🎧 [BOOTLOADER] Current flow state:', flowStateRef.current);
      
      // Test that events work at all
      console.log('🧪 [BOOTLOADER] Testing if Tauri events work...');
      
      // Listen for device disconnect
      console.log('🎧 [BOOTLOADER] Setting up device:disconnected listener...');
      const disconnectListener = await listen('device:disconnected', (event: any) => {
        console.log('🔌 [BOOTLOADER] ==================== DISCONNECT EVENT RECEIVED ====================');
        console.log('🔌 [BOOTLOADER] Event payload:', event.payload);
        console.log('🔌 [BOOTLOADER] Current state:', flowStateRef.current);
        console.log('🔌 [BOOTLOADER] Our device ID:', deviceId);
        console.log('🔌 [BOOTLOADER] ================================================================');
        
        const disconnectedDeviceId = event.payload?.device_id || event.payload;
        
        if (disconnectedDeviceId === deviceId) {
          console.log('✅ [BOOTLOADER] Our device disconnected! Device ID matches:', deviceId);
          if (flowStateRef.current === 'wait_disconnect') {
            console.log('✅ [BOOTLOADER] Step 1 completed - Device unplugged');
            setCompletedSteps(prev => ({ ...prev, step1_unplugged: true, step2_holding: true }));
            setFlowState('disconnected');
          } else {
            console.log(`⚠️ [BOOTLOADER] Device disconnected but we're in state "${flowStateRef.current}", not waiting for disconnect`);
          }
        } else {
          console.log(`ℹ️ [BOOTLOADER] Different device disconnected: ${disconnectedDeviceId} (ours: ${deviceId})`);
        }
      });

      // Listen for device reconnect
      console.log('🎧 [BOOTLOADER] Setting up device:connected listener...');
      const connectListener = await listen('device:connected', (event: any) => {
        console.log('🔌 [BOOTLOADER] ==================== CONNECT EVENT RECEIVED ====================');
        console.log('🔌 [BOOTLOADER] Event payload:', event.payload);
        console.log('🔌 [BOOTLOADER] Current state:', flowStateRef.current);
        console.log('🔌 [BOOTLOADER] Our device ID:', deviceId);
        console.log('🔌 [BOOTLOADER] ================================================================');
        
        const connectedDeviceId = event.payload?.unique_id || event.payload;
        
        if (connectedDeviceId === deviceId) {
          console.log('🔄 [BOOTLOADER] Our device reconnected!', deviceId);
          
          // Always check for bootloader mode when our device reconnects
          setCompletedSteps(prev => ({ ...prev, step3_plugged: true }));
          
          // Check if device is in bootloader mode
          setTimeout(async () => {
            try {
              console.log('🔍 [BOOTLOADER] Checking if device is in bootloader mode...');
              const features = await invoke('get_features', { deviceId }) as DeviceFeatures;
              console.log('🔍 [BOOTLOADER] Reconnected device features:', features);
              
              if (features && features.bootloaderMode) {
                console.log('🎯 [BOOTLOADER] Device is in bootloader mode! All steps complete!');
                setCompletedSteps(prev => ({ ...prev, step4_bootloader: true }));
                
                // Wait a moment to show completion, then auto-advance
                setTimeout(() => {
                  setFlowState('bootloader_ready');
                }, 1500);
              } else {
                console.log('❌ [BOOTLOADER] Device not in bootloader mode, staying in current state');
                // Don't reset if we're already in a good state
                if (flowStateRef.current === 'wait_disconnect' || flowStateRef.current === 'instructions') {
                  console.log('🔄 [BOOTLOADER] Back to instructions');
                  setFlowState('instructions');
                  // Reset steps since bootloader mode wasn't achieved
                  setCompletedSteps({
                    step1_unplugged: false,
                    step2_holding: false,
                    step3_plugged: false,
                    step4_bootloader: false,
                  });
                }
              }
            } catch (error) {
              console.error('Failed to check device features after reconnect:', error);
            }
          }, 2000); // Give device time to settle
        } else {
          console.log(`ℹ️ [BOOTLOADER] Different device connected: ${connectedDeviceId} (ours: ${deviceId})`);
        }
      });

      // Store listeners for cleanup
      (window as any).bootloaderListeners = {
        disconnect: disconnectListener,
        connect: connectListener
      };
      
      console.log('✅ [BOOTLOADER] Initial event listeners set up successfully');
      console.log('✅ [BOOTLOADER] Stored listeners in window.bootloaderListeners');
      console.log('✅ [BOOTLOADER] Setup complete - ready to receive events!');
      
    } catch (error) {
      console.error('Failed to set up initial bootloader event listeners:', error);
      setError('Failed to set up device monitoring');
    }
  };

  // Cleanup event listeners
  useEffect(() => {
    return () => {
      console.log('🧹 [BOOTLOADER] Cleaning up event listeners');
      const listeners = (window as any).bootloaderListeners;
      if (listeners) {
        if (listeners.disconnect && typeof listeners.disconnect === 'function') {
          listeners.disconnect();
        }
        if (listeners.connect && typeof listeners.connect === 'function') {
          listeners.connect();
        }
        delete (window as any).bootloaderListeners;
      }
    };
  }, []);

  // Auto-cleanup when we reach complete state
  useEffect(() => {
    if (flowState === 'complete' || flowState === 'bootloader_ready') {
      console.log('🧹 [BOOTLOADER] Flow complete, cleaning up listeners');
      const listeners = (window as any).bootloaderListeners;
      if (listeners) {
        if (listeners.disconnect && typeof listeners.disconnect === 'function') {
          listeners.disconnect();
        }
        if (listeners.connect && typeof listeners.connect === 'function') {
          listeners.connect();
        }
        delete (window as any).bootloaderListeners;
      }
    }
  }, [flowState]);

  const checkReconnectedDevice = async (newDeviceId: string) => {
    try {
      console.log(`🔍 [BOOTLOADER] Checking device ${newDeviceId} for bootloader mode...`);
      const result = await invoke('get_features', { deviceId: newDeviceId });
      console.log('🔍 [BOOTLOADER] Reconnected device features:', result);
      
      const deviceFeatures = result as DeviceFeatures;
      console.log(`🔍 [BOOTLOADER] bootloader_mode: ${deviceFeatures.bootloaderMode}`);
      console.log(`🔍 [BOOTLOADER] firmware_version: ${deviceFeatures.version}`);
      console.log(`🔍 [BOOTLOADER] bootloader_version: ${deviceFeatures.bootloaderVersion}`);
      
      if (deviceFeatures.bootloaderMode === true) {
        console.log('✅ [BOOTLOADER] Device reconnected in bootloader mode!');
        setFeatures(deviceFeatures);
        setFlowState('bootloader_ready');
      } else {
        console.log('⚠️ [BOOTLOADER] Device reconnected but bootloader_mode is:', deviceFeatures.bootloaderMode);
        // Don't error immediately - maybe device needs more time
        console.log('⏳ [BOOTLOADER] Retrying in 2 seconds...');
        setTimeout(() => {
          checkReconnectedDevice(newDeviceId);
        }, 2000);
      }
    } catch (err) {
      console.error('❌ [BOOTLOADER] Failed to check reconnected device:', err);
      // Also retry on error - device might not be ready yet
      console.log('⏳ [BOOTLOADER] Retrying in 2 seconds due to error...');
      setTimeout(() => {
        checkReconnectedDevice(newDeviceId);
      }, 2000);
    }
  };

  const checkAllDevicesForBootloader = async () => {
    try {
      console.log('🔍 [BOOTLOADER] Checking all connected devices for bootloader mode...');
      const connectedDevices = await invoke('get_connected_devices');
      console.log('🔍 [BOOTLOADER] Connected devices:', connectedDevices);
      
      if (Array.isArray(connectedDevices)) {
        for (const device of connectedDevices) {
          if (device.device_id && device.device_id !== deviceId) {
            console.log(`🔍 [BOOTLOADER] Checking device ${device.device_id} for bootloader mode...`);
            try {
              const result = await invoke('get_features', { deviceId: device.device_id });
              const deviceFeatures = result as DeviceFeatures;
              
              if (deviceFeatures.bootloaderMode === true) {
                console.log(`✅ [BOOTLOADER] Found device ${device.device_id} in bootloader mode!`);
                setFeatures(deviceFeatures);
                setFlowState('bootloader_ready');
                return;
              }
            } catch (err) {
              console.log(`⚠️ [BOOTLOADER] Failed to check device ${device.device_id}:`, err);
            }
          }
        }
      }
    } catch (err) {
      console.log('⚠️ [BOOTLOADER] Failed to get connected devices:', err);
    }
  };

  const getDeviceFeatures = async () => {
    try {
      setFlowState('checking');
      setError(null);
      
      const result = await invoke('get_features', { deviceId });
      console.log('Raw device features:', result);
      
      const deviceFeatures = result as DeviceFeatures;
      setFeatures(deviceFeatures);
      
      // Check if bootloader needs update
      console.log(`🔍 [BOOTLOADER] Comparing versions: current="${deviceFeatures.bootloaderVersion}" vs latest="${LATEST_BOOTLOADER_VERSION}"`);
      
      if (deviceFeatures.bootloaderVersion && deviceFeatures.bootloaderVersion !== LATEST_BOOTLOADER_VERSION) {
        console.log('⚠️ [BOOTLOADER] Device needs bootloader update!');
        setFlowState('needs_update');
      } else if (deviceFeatures.bootloaderVersion === LATEST_BOOTLOADER_VERSION) {
        console.log('✅ [BOOTLOADER] Device has latest bootloader!');
        setFlowState('complete');
      } else {
        console.log('❌ [BOOTLOADER] Could not determine bootloader version');
        setError('Could not determine bootloader version');
        setFlowState('error');
      }
    } catch (err) {
      console.error('Failed to get features:', err);
      setError(err?.toString() || 'Failed to get device features');
      setFlowState('error');
    }
  };

  const startBootloaderUpdate = () => {
    setFlowState('instructions');
  };

  const startWaitingForDisconnect = () => {
    console.log('🔄 [BOOTLOADER] User clicked "I\'m Ready" - now waiting for disconnect...');
    setFlowState('wait_disconnect');
  };

  const startWaitingForReconnect = () => {
    setFlowState('wait_reconnect');
    
    // Start backup polling in case events don't work
    console.log('🔄 [BOOTLOADER] Starting backup polling for device reconnection...');
    startBackupPolling();
  };

  const startBackupPolling = () => {
    let pollCount = 0;
    const maxPolls = 30; // Poll for up to 60 seconds
    
    const poll = async () => {
      if (flowState !== 'wait_reconnect') {
        console.log('🛑 [BOOTLOADER] Stopping backup polling - state changed');
        return;
      }
      
      pollCount++;
      console.log(`🔄 [BOOTLOADER] Backup poll ${pollCount}/${maxPolls} - checking for bootloader device...`);
      
             try {
         // Try to check the original device ID first
         await checkReconnectedDevice(deviceId!);
         
         // Also try to get all connected devices and check if any are in bootloader mode
         await checkAllDevicesForBootloader();
         
       } catch (err) {
        console.log(`⚠️ [BOOTLOADER] Backup poll ${pollCount} failed:`, err);
      }
      
      if (pollCount < maxPolls && flowState === 'wait_reconnect') {
        setTimeout(poll, 2000);
      } else if (pollCount >= maxPolls) {
        console.log('❌ [BOOTLOADER] Backup polling timeout - device not found');
        setError('Device not detected in bootloader mode. Please try the process again.');
        setFlowState('error');
      }
    };
    
    // Start polling after a short delay
    setTimeout(poll, 3000);
  };

  const performBootloaderUpdate = async () => {
    if (!features) return;
    
    try {
      setFlowState('updating');
      setError(null);
      
      const success = await invoke('update_device_bootloader', {
        deviceId: deviceId,
        targetVersion: LATEST_BOOTLOADER_VERSION
      });
      
      if (success) {
        setFlowState('complete');
      } else {
        setError('Bootloader update failed');
        setFlowState('error');
      }
    } catch (err) {
      console.error('Bootloader update failed:', err);
      setError(err?.toString() || 'Update failed');
      setFlowState('error');
    }
  };

  const handleContinue = () => {
    onNext({
      bootloaderChecked: true,
      bootloaderUpdated: flowState === 'complete',
      bootloaderVersion: features?.bootloaderVersion,
      bootloaderSecure: flowState === 'complete',
    });
  };

  const renderFlowState = () => {
    switch (flowState) {
      case 'checking':
        return (
          <VStack gap={4} py={8}>
            <Spinner size="lg" color="blue.400" />
            <Text>Checking bootloader security...</Text>
            
            {/* Debug info */}
            {features && (
              <Box p={3} bg="gray.800" borderRadius="md" fontSize="xs" color="gray.400">
                <Text>Debug: bootloader_version = "{features.bootloaderVersion}"</Text>
                <Text>Debug: LATEST_BOOTLOADER_VERSION = "{LATEST_BOOTLOADER_VERSION}"</Text>
                <Text>Debug: flowState = "{flowState}"</Text>
                <Text>Debug: error = "{error}"</Text>
              </Box>
            )}
          </VStack>
        );

      case 'needs_update':
        return (
          <VStack gap={6} align="stretch">
            <Box p={6} bg="blue.900" borderRadius="md" borderWidth="1px" borderColor="blue.500">
              <HStack justify="space-between" mb={4}>
                <HStack>
                  <Icon as={FaCheckCircle} boxSize={6} color="blue.400" />
                  <Text fontSize="lg" fontWeight="bold" color="blue.200">
                    🔧 Device Setup Required
                  </Text>
                </HStack>
                <Badge colorScheme="blue" size="lg">SETUP</Badge>
              </HStack>

              <VStack gap={3} align="stretch">
                <Box>
                  <Text fontSize="sm" color="gray.400" mb={1}>Current Bootloader Version:</Text>
                  <Code colorScheme="gray" fontSize="sm">
                    v{features?.bootloaderVersion}
                  </Code>
                </Box>
                <Box>
                  <Text fontSize="sm" color="gray.400" mb={1}>Target Version:</Text>
                  <Code colorScheme="blue" fontSize="sm">v{LATEST_BOOTLOADER_VERSION}</Code>
                </Box>
                
                <Text color="blue.300" fontSize="sm">
                  Welcome! To complete your KeepKey setup, we'll update your device to the latest firmware. 
                  This ensures you have access to all the newest features and optimal performance.
                </Text>

                <Button colorScheme="blue" onClick={startBootloaderUpdate} size="lg" mt={2}>
                  ⚡ Continue Setup
                </Button>
              </VStack>
            </Box>
          </VStack>
        );

      case 'instructions':
        return (
          <VStack gap={6} align="stretch">
            <Box p={6} bg="blue.900" borderRadius="md" borderWidth="1px" borderColor="blue.500">
              <Text fontSize="lg" fontWeight="bold" color="blue.200" mb={6} textAlign="center">
                📋 Put Your Device in Bootloader Mode
              </Text>
              
              <HStack gap={6} align="flex-start">
                {/* GIF Section */}
                <Box flex="1" maxW="400px">
                  <Box p={4} bg="gray.800" borderRadius="md" textAlign="center">
                    <Text color="gray.400" fontSize="sm" mb={2}>Bootloader Mode Instructions</Text>
                    <Box h="280px" bg="gray.700" borderRadius="md" display="flex" alignItems="center" justifyContent="center" overflow="hidden">
                      <Image 
                        src={bootloaderGif} 
                        alt="KeepKey bootloader mode instructions"
                        maxH="260px"
                        maxW="100%"
                        objectFit="contain"
                        borderRadius="md"
                      />
                    </Box>
                  </Box>
                </Box>

                {/* Instructions Section */}
                <Box flex="1" minW="300px">
                  <VStack gap={4} align="stretch">
                    <Text fontWeight="bold" color="blue.100" fontSize="md" mb={2}>
                      Follow these simple steps:
                    </Text>
                    
                    <VStack gap={3} fontSize="sm" color="blue.100" align="stretch">
                      <HStack>
                        <Icon 
                          as={completedSteps.step1_unplugged ? FaCheck : FaArrowDown} 
                          color={completedSteps.step1_unplugged ? "green.400" : "blue.400"} 
                          minW="16px" 
                        />
                        <Text 
                          textDecoration={completedSteps.step1_unplugged ? "line-through" : "none"}
                          opacity={completedSteps.step1_unplugged ? 0.7 : 1}
                        >
                          <strong>1.</strong> UNPLUG your KeepKey from the USB cable
                        </Text>
                      </HStack>
                      <HStack>
                        <Icon 
                          as={completedSteps.step2_holding ? FaCheck : FaArrowDown} 
                          color={completedSteps.step2_holding ? "green.400" : "blue.400"} 
                          minW="16px" 
                        />
                        <Text 
                          textDecoration={completedSteps.step2_holding ? "line-through" : "none"}
                          opacity={completedSteps.step2_holding ? 0.7 : 1}
                        >
                          <strong>2.</strong> HOLD DOWN the single button on your device
                        </Text>
                      </HStack>
                      <HStack>
                        <Icon 
                          as={completedSteps.step3_plugged ? FaCheck : FaArrowDown} 
                          color={completedSteps.step3_plugged ? "green.400" : "blue.400"} 
                          minW="16px" 
                        />
                        <Text 
                          textDecoration={completedSteps.step3_plugged ? "line-through" : "none"}
                          opacity={completedSteps.step3_plugged ? 0.7 : 1}
                        >
                          <strong>3.</strong> While continuing to hold the button, PLUG the USB cable back in
                        </Text>
                      </HStack>
                      <HStack>
                        <Icon 
                          as={completedSteps.step4_bootloader ? FaCheck : FaArrowDown} 
                          color={completedSteps.step4_bootloader ? "green.400" : "blue.400"} 
                          minW="16px" 
                        />
                        <Text 
                          textDecoration={completedSteps.step4_bootloader ? "line-through" : "none"}
                          opacity={completedSteps.step4_bootloader ? 0.7 : 1}
                        >
                          <strong>4.</strong> Keep holding until the screen shows "BOOTLOADER MODE"
                        </Text>
                      </HStack>
                    </VStack>

                    <Button colorScheme="blue" onClick={startWaitingForDisconnect} size="lg" mt={4}>
                      ✅ I'm Ready - Start Monitoring
                    </Button>
                  </VStack>
                </Box>
              </HStack>
            </Box>
          </VStack>
        );

      case 'wait_disconnect':
        return (
          <VStack gap={4} py={8}>
            <Spinner size="lg" color="blue.400" />
            <Text fontSize="lg" fontWeight="bold">Waiting for device to be unplugged...</Text>
            <Text color="gray.400" textAlign="center">
              Please follow the instructions above to unplug your device
            </Text>
          </VStack>
        );

      case 'disconnected':
        return (
          <VStack gap={4} py={8}>
            <Icon as={FaCheckCircle} boxSize={8} color="green.400" />
            <Text fontSize="lg" fontWeight="bold" color="green.400">Device Unplugged Successfully!</Text>
            <Text color="gray.400" textAlign="center">
              Great! Now hold the button and plug it back in to enter bootloader mode...
            </Text>
          </VStack>
        );

      case 'wait_reconnect':
        return (
          <VStack gap={4} py={8}>
            <Icon as={FaUsb} boxSize={12} color="blue.400" />
            <Text fontSize="lg" fontWeight="bold">Waiting for device to reconnect...</Text>
            <Text color="gray.400" textAlign="center">
              Plug back in while holding both buttons for 3 seconds until screen shows "BOOTLOADER"
            </Text>
            <Spinner size="lg" color="blue.400" />
            
            <VStack gap={2} mt={4}>
              <Text fontSize="sm" color="gray.500">
                Device not detected automatically?
              </Text>
              <Button 
                variant="outline" 
                size="sm" 
                onClick={async () => {
                  console.log('🔍 [BOOTLOADER] Manual check triggered');
                  await checkReconnectedDevice(deviceId!);
                  await checkAllDevicesForBootloader();
                }}
              >
                🔍 Check Device Manually
              </Button>
            </VStack>
          </VStack>
        );

      case 'bootloader_ready':
        return (
          <VStack gap={6} align="stretch">
            <Box p={6} bg="green.900" borderRadius="md" borderWidth="1px" borderColor="green.500">
              <HStack mb={4}>
                <Icon as={FaCheckCircle} boxSize={6} color="green.400" />
                <Text fontSize="lg" fontWeight="bold" color="green.200">
                  🎯 Device Ready for Update!
                </Text>
              </HStack>
              
              <VStack gap={3} align="stretch">
                <Text color="green.300">
                  Your KeepKey is now in bootloader mode and ready for the security update.
                </Text>
                
                <Box>
                  <Text fontSize="sm" color="gray.400" mb={1}>Device Status:</Text>
                  <Code colorScheme="green">BOOTLOADER MODE ✅</Code>
                </Box>

                <Button colorScheme="green" onClick={performBootloaderUpdate} size="lg" mt={2}>
                  🔄 Update Bootloader Now
                </Button>
              </VStack>
            </Box>
          </VStack>
        );

      case 'updating':
        return (
          <VStack gap={4} py={8}>
            <Spinner size="lg" color="blue.400" />
            <Text fontSize="lg" fontWeight="bold">Updating bootloader...</Text>
            <Text color="gray.400" textAlign="center">
              Please do not disconnect your device during the update
            </Text>
          </VStack>
        );

      case 'complete':
        return (
          <VStack gap={6} align="stretch">
            <Box p={6} bg="green.900" borderRadius="md" borderWidth="1px" borderColor="green.500">
              <HStack mb={4}>
                <Icon as={FaCheckCircle} boxSize={6} color="green.400" />
                <Text fontSize="lg" fontWeight="bold" color="green.200">
                  🎉 Security Update Complete!
                </Text>
              </HStack>
              
              <VStack gap={3} align="stretch">
                <Text color="green.300">
                  Your KeepKey now has the latest secure bootloader v{LATEST_BOOTLOADER_VERSION}.
                  Your device is protected against known security vulnerabilities.
                </Text>
                
                <Box>
                  <Text fontSize="sm" color="gray.400" mb={1}>New Bootloader Version:</Text>
                  <Code colorScheme="green">v{LATEST_BOOTLOADER_VERSION} ✅ SECURE</Code>
                </Box>
              </VStack>
            </Box>
          </VStack>
        );

      case 'error':
        return (
          <VStack gap={4} py={8}>
            <Icon as={FaExclamationTriangle} boxSize={12} color="red.400" />
            <Text fontSize="lg" fontWeight="bold" color="red.300">
              Update Error
            </Text>
            <Text color="red.300" textAlign="center">
              {error}
            </Text>
            <Button colorScheme="red" onClick={getDeviceFeatures}>
              Retry
            </Button>
          </VStack>
        );

      default:
        return null;
    }
  };

  return (
    <Box maxWidth="800px" margin="auto">
      <Card.Root size="lg">
        <Card.Header>
          <HStack>
            <Icon as={FaShieldAlt} boxSize={6} color="blue.400" />
            <VStack align="flex-start" gap={1}>
              <Text fontSize="xl" fontWeight="bold">Verify Bootloader Security</Text>
              <Text fontSize="sm" color="gray.400">
                Ensuring your device has the latest security firmware
              </Text>
            </VStack>
          </HStack>
        </Card.Header>

        <Card.Body>
          {renderFlowState()}
        </Card.Body>

        <Card.Footer>
          <HStack justify="space-between" width="100%">
            <HStack>
              {/* Debug button */}
              <Button 
                variant="outline" 
                size="sm" 
                onClick={async () => {
                  console.log('🔧 [DEBUG] Force checking device for bootloader mode...');
                  if (deviceId) {
                    await checkReconnectedDevice(deviceId);
                    await checkAllDevicesForBootloader();
                  }
                }}
              >
                🔧 Debug Check
              </Button>
            </HStack>
            
            <Button 
              colorScheme="green"
              onClick={handleContinue}
              disabled={flowState !== 'complete'}
            >
              {flowState === 'complete' ? 'Continue Setup' : 'Update Required'}
              {flowState === 'complete' && <Icon as={FaCheckCircle} ml={2} />}
            </Button>
          </HStack>
        </Card.Footer>
      </Card.Root>
    </Box>
  );
} 