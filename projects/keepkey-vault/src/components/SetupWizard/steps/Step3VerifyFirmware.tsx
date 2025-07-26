import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Box,
  Button,
  Text,
  VStack,
  HStack,
  Progress,
  Spinner,
  Badge,
  Heading,
  List,
  ListItem,
  Icon
} from '@chakra-ui/react';
import { FaDownload, FaCheckCircle, FaExclamationTriangle } from "react-icons/fa";
import { EnterBootloaderModeDialog } from '../../EnterBootloaderModeDialog';

interface StepProps {
  onNext: (data?: any) => void;
  onSkip?: () => void;
  deviceId?: string;
}

interface DeviceStatus {
  deviceId: string;
  connected: boolean;
  features?: {
    bootloaderMode?: boolean;
  };
  firmwareCheck?: {
    currentVersion: string;
    latestVersion: string;
    needsUpdate: boolean;
  };
  needsFirmwareUpdate: boolean;
}

export function Step3VerifyFirmware({ onNext, onSkip, deviceId }: StepProps) {
  const [loading, setLoading] = useState(true);
  const [deviceStatus, setDeviceStatus] = useState<DeviceStatus | null>(null);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [updateComplete, setUpdateComplete] = useState(false);
  const [showBootloaderInstructions, setShowBootloaderInstructions] = useState(false);
  const [waitingForBootloaderMode, setWaitingForBootloaderMode] = useState(false);

  useEffect(() => {
    checkFirmware();
    setupEventListeners();
    
    // Cleanup on unmount
    return () => {
      // Clear recovery flag if component unmounts
      if ((window as any).KEEPKEY_RECOVERY_IN_PROGRESS) {
        (window as any).KEEPKEY_RECOVERY_IN_PROGRESS = false;
        console.log("🔓 GLOBAL RECOVERY LOCK DISABLED on component unmount");
      }
      
      // Cleanup event listeners
      if ((window as any).step3Listeners) {
        const { connectUnsubscribe, featuresUnsubscribe } = (window as any).step3Listeners;
        if (connectUnsubscribe) connectUnsubscribe();
        if (featuresUnsubscribe) featuresUnsubscribe();
        delete (window as any).step3Listeners;
      }
    };
  }, []);

  const setupEventListeners = async () => {
    console.log('🎧 [Step3] Setting up device event listeners');
    
    // Listen for device connection events
    const connectUnsubscribe = await listen('device:connected', (event: any) => {
      console.log('🔌 [Step3] Device connected event:', event.payload);
      if (event.payload.is_keepkey && waitingForBootloaderMode) {
        console.log('🔄 [Step3] KeepKey reconnected, checking if in bootloader mode...');
        setTimeout(() => {
          checkFirmware();
        }, 1000);
      }
    });

    // Listen for device features updates
    const featuresUnsubscribe = await listen('device:features-updated', (event: any) => {
      console.log('🔧 [Step3] Device features updated:', event.payload);
      const { status } = event.payload;
      
      if (status.deviceId === deviceId) {
        setDeviceStatus(status);
        
        // If we were waiting for bootloader mode and now we have it
        if (waitingForBootloaderMode && status.features?.bootloaderMode === true) {
          console.log('✅ [Step3] Device now in bootloader mode, hiding instructions');
          setShowBootloaderInstructions(false);
          setWaitingForBootloaderMode(false);
        }
      }
    });

    // Store unsubscribe functions for cleanup
    (window as any).step3Listeners = {
      connectUnsubscribe,
      featuresUnsubscribe
    };
  };

  const checkFirmware = async () => {
    if (!deviceId) return;
    
    try {
      setLoading(true);
      setError(null);
      
      // Use get_device_status instead of separate check_device_firmware
      const status = await invoke<DeviceStatus>('get_device_status', { deviceId });
      setDeviceStatus(status);
    } catch (error) {
      console.error('Failed to check firmware:', error);
      setError(error?.toString() || 'Failed to check firmware status');
    } finally {
      setLoading(false);
    }
  };

  const handleUpdate = async () => {
    if (!deviceId || !deviceStatus?.firmwareCheck) return;
    
    try {
      setUpdating(true);
      setError(null);
      
      // Check if device is in bootloader mode first
      const isInBootloaderMode = deviceStatus.features?.bootloaderMode === true;
      console.log('🔧 [Step3] Device in bootloader mode:', isInBootloaderMode);
      
      if (!isInBootloaderMode) {
        // Device needs to enter bootloader mode first
        console.log('🔧 [Step3] Device not in bootloader mode, showing instructions');
        setUpdating(false);
        setShowBootloaderInstructions(true);
        setWaitingForBootloaderMode(true);
        return;
      }
      
      console.log('🔧 [Step3] Device in bootloader mode, proceeding with firmware update');
      
      // Set global recovery flag to prevent UI interference
      (window as any).KEEPKEY_RECOVERY_IN_PROGRESS = true;
      console.log("🛡️ GLOBAL RECOVERY LOCK ENABLED for firmware update");
      
      // Call backend to update firmware
      const success = await invoke<boolean>('update_device_firmware', {
        deviceId,
        targetVersion: deviceStatus.firmwareCheck.latestVersion
      });
      
      if (success) {
        setUpdateComplete(true);
        // Clear recovery flag on success
        (window as any).KEEPKEY_RECOVERY_IN_PROGRESS = false;
        console.log("🔓 GLOBAL RECOVERY LOCK DISABLED after successful update");
        
        // Re-check firmware status after update
        setTimeout(() => {
          checkFirmware();
        }, 3000);
      }
    } catch (error) {
      console.error('Firmware update failed:', error);
      setError(error?.toString() || 'Update failed');
      // Clear recovery flag on error
      (window as any).KEEPKEY_RECOVERY_IN_PROGRESS = false;
      console.log("🔓 GLOBAL RECOVERY LOCK DISABLED after failed update");
    } finally {
      setUpdating(false);
    }
  };

  const handleContinue = () => {
    onNext({ 
      firmwareChecked: true,
      firmwareUpdated: updateComplete,
      firmwareStatus: deviceStatus?.firmwareCheck
    });
  };

  const handleSkip = () => {
    if (onSkip) {
      onSkip();
    } else {
      onNext({ skipped: true });
    }
  };

  return (
    <>
      <Box width="full" maxWidth="2xl">
        <Box bg="gray.900" borderColor="gray.700" borderRadius="md" p={6}>
          <HStack justify="center" gap={3}>
            <Icon as={FaDownload} color="blue.500" boxSize={8} />
            <Heading fontSize="xl" fontWeight="bold" color="white">
              Verify Firmware
            </Heading>
          </HStack>
          
          <VStack gap={6} mt={6}>
            <Text color="gray.400" textAlign="center">
              Checking your device's firmware for the latest features and security updates.
            </Text>
            
            {/* Show waiting message when bootloader instructions are displayed */}
            {showBootloaderInstructions ? (
              <VStack gap={4} p={6} bg="yellow.900" borderRadius="md" borderWidth="1px" borderColor="yellow.600">
                <Icon as={FaExclamationTriangle} boxSize={12} color="yellow.400" />
                <Text fontSize="lg" fontWeight="semibold" color="white">
                  Waiting for Bootloader Mode
                </Text>
                <Text color="yellow.200" textAlign="center">
                  Please follow the instructions in the dialog to put your device in bootloader mode.
                </Text>
                <Text fontSize="sm" color="gray.400" textAlign="center">
                  The firmware update will continue automatically once your device is in bootloader mode.
                </Text>
                <HStack gap={4} mt={4}>
                  <Button
                    variant="outline"
                    colorScheme="gray"
                    onClick={() => {
                      setShowBootloaderInstructions(false);
                      setWaitingForBootloaderMode(false);
                    }}
                  >
                    Cancel
                  </Button>
                  <Button
                    colorScheme="yellow"
                    onClick={() => {
                      // Re-show instructions
                      setShowBootloaderInstructions(true);
                    }}
                  >
                    Show Instructions Again
                  </Button>
                </HStack>
              </VStack>
            ) : loading ? (
              <VStack gap={4}>
                <Spinner size="lg" color="blue.500" />
                <Text color="gray.400">Checking firmware status...</Text>
              </VStack>
            ) : error ? (
              <VStack gap={4} p={6} bg="red.900" borderRadius="md" borderWidth="1px" borderColor="red.600">
                <Icon as={FaExclamationTriangle} boxSize={12} color="red.400" />
                <Text fontSize="lg" fontWeight="semibold" color="white">
                  Firmware Check Failed
                </Text>
                <Text color="red.200" textAlign="center">
                  {error}
                </Text>
                <HStack gap={4}>
                  <Button onClick={checkFirmware} colorScheme="red" variant="outline">
                    Retry
                  </Button>
                  <Button onClick={handleSkip} variant="outline">
                    Skip This Step
                  </Button>
                </HStack>
              </VStack>
            ) : deviceStatus ? (
              <VStack gap={6} width="full">
                {/* Check if device is disconnected or we have invalid firmware version data */}
                {!deviceStatus.connected || !deviceStatus.firmwareCheck?.currentVersion || !deviceStatus.firmwareCheck?.latestVersion ? (
                  <VStack gap={4} p={6} bg="red.900" borderRadius="md" borderWidth="1px" borderColor="red.600">
                    <Icon as={FaExclamationTriangle} boxSize={12} color="red.400" />
                    <Text fontSize="lg" fontWeight="semibold" color="white">
                      {!deviceStatus.connected ? 'Device Connection Lost' : 'Cannot Read Device Firmware'}
                    </Text>
                    <Text color="red.200" textAlign="center">
                      {!deviceStatus.connected 
                        ? 'Your device appears to be disconnected. Please reconnect your KeepKey and try again.'
                        : 'Unable to communicate with your device to check firmware version. Please ensure your device is properly connected.'
                      }
                    </Text>
                    <VStack gap={1} fontSize="sm" color="gray.400">
                      <Text>Device Status: {deviceStatus.connected ? 'Connected' : 'Disconnected'}</Text>
                      <Text>Current Version: {deviceStatus.firmwareCheck?.currentVersion || 'Unknown'}</Text>
                      <Text>Latest Version: {deviceStatus.firmwareCheck?.latestVersion || 'Unknown'}</Text>
                    </VStack>
                    <HStack gap={4}>
                      <Button onClick={checkFirmware} colorScheme="red" variant="outline">
                        {!deviceStatus.connected ? 'Check Connection' : 'Retry Connection'}
                      </Button>
                      <Button onClick={handleSkip} variant="outline">
                        Skip This Step
                      </Button>
                    </HStack>
                  </VStack>
                ) : deviceStatus.needsFirmwareUpdate ? (
                  <Box w="full" p={6} bg="blue.900" borderRadius="md" borderWidth="1px" borderColor="blue.600">
                    <VStack gap={4}>
                      <HStack gap={3}>
                        <Icon as={FaDownload} color="blue.400" boxSize={6} />
                        <Text fontSize="lg" fontWeight="bold" color="white">
                          Firmware Update Available
                        </Text>
                        <Badge colorScheme="blue">
                          Update Available
                        </Badge>
                      </HStack>
                      
                      <HStack gap={8} width="full" justify="space-around">
                        <VStack>
                          <Text fontSize="sm" color="gray.400">Current Version</Text>
                          <Text fontSize="lg" fontWeight="semibold">{deviceStatus.firmwareCheck?.currentVersion}</Text>
                        </VStack>
                        <Icon as={FaDownload} color="blue.400" boxSize={8} />
                        <VStack>
                          <Text fontSize="sm" color="gray.400">Latest Version</Text>
                          <Text fontSize="lg" fontWeight="semibold" color="green.400">
                            {deviceStatus.firmwareCheck?.latestVersion}
                          </Text>
                        </VStack>
                      </HStack>
                      
                      <Text color="blue.200" textAlign="center" fontSize="sm">
                        This update includes new features and security improvements.
                      </Text>
                      
                      {updating ? (
                        <VStack gap={2}>
                          <Spinner color="blue.400" />
                          <Text color="blue.200">Updating firmware...</Text>
                          <Text fontSize="sm" color="gray.400">This may take 3-5 minutes</Text>
                        </VStack>
                      ) : updateComplete ? (
                        <VStack gap={2}>
                          <Icon as={FaCheckCircle} color="green.400" boxSize={6} />
                          <Text color="green.200">Firmware updated successfully!</Text>
                        </VStack>
                      ) : (
                        <VStack gap={4} width="full">
                          <VStack gap={2}>
                            <Text fontSize="sm" fontWeight="bold" color="yellow.400">
                              Important:
                            </Text>
                            <Text fontSize="sm" color="gray.300" textAlign="center">
                              • Do not disconnect your device during the update
                            </Text>
                            <Text fontSize="sm" color="gray.300" textAlign="center">
                              • You may need to re-enter your PIN after the update
                            </Text>
                            <Text fontSize="sm" color="gray.300" textAlign="center">
                              • Your funds and settings will remain safe
                            </Text>
                          </VStack>
                          
                          <HStack gap={4}>
                            <Button onClick={handleSkip} variant="outline" colorScheme="gray">
                              Skip Update
                            </Button>
                            <Button
                              onClick={handleUpdate}
                              colorScheme="blue"
                              size="lg"
                              loading={updating}
                              loadingText="Updating..."
                            >
                              Update Firmware
                            </Button>
                          </HStack>
                        </VStack>
                      )}
                    </VStack>
                  </Box>
                ) : (
                  <VStack gap={4} p={6} bg="green.900" borderRadius="md" borderWidth="1px" borderColor="green.600">
                    <Icon as={FaCheckCircle} boxSize={12} color="green.400" />
                    <Text fontSize="lg" fontWeight="semibold" color="white">
                      Firmware is Up to Date
                    </Text>
                    <Text color="green.200" textAlign="center">
                      Your device is running the latest firmware with all current features and security updates.
                    </Text>
                    <VStack gap={1}>
                      <Text fontSize="sm" color="gray.400">
                        Current Version: {deviceStatus.firmwareCheck?.currentVersion}
                      </Text>
                      <Text fontSize="sm" color="gray.400">
                        Latest Version: {deviceStatus.firmwareCheck?.latestVersion}
                      </Text>
                    </VStack>
                  </VStack>
                )}
                
                <Button
                  onClick={handleContinue}
                  colorScheme="blue"
                  size="lg"
                  width="full"
                  disabled={deviceStatus.needsFirmwareUpdate && !updateComplete}
                >
                  Continue to Wallet Setup
                </Button>
              </VStack>
            ) : null}
          </VStack>
        </Box>
      </Box>
      
      {/* Bootloader Mode Instructions Dialog */}
      {showBootloaderInstructions && deviceStatus?.firmwareCheck && (
        <EnterBootloaderModeDialog
          isOpen={showBootloaderInstructions}
          updateType="firmware"
          firmwareCheck={deviceStatus.firmwareCheck}
          onClose={() => {
            setShowBootloaderInstructions(false);
            setWaitingForBootloaderMode(false);
            // Re-check device status after user closes dialog
            setTimeout(() => {
              checkFirmware();
            }, 1000);
          }}
        />
      )}
      </>
    );
  } 