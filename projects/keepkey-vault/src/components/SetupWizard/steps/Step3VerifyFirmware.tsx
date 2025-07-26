import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
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

interface StepProps {
  onNext: (data?: any) => void;
  onSkip?: () => void;
  deviceId?: string;
}

interface DeviceStatus {
  deviceId: string;
  connected: boolean;
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

  useEffect(() => {
    if (deviceId) {
      checkFirmware();
    }
  }, [deviceId]);

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
      
      // Call backend to update firmware
      const success = await invoke<boolean>('update_device_firmware', {
        deviceId,
        targetVersion: deviceStatus.firmwareCheck.latestVersion
      });
      
      if (success) {
        setUpdateComplete(true);
        // Re-check firmware status after update
        setTimeout(() => {
          checkFirmware();
        }, 3000);
      }
    } catch (error) {
      console.error('Firmware update failed:', error);
      setError(error?.toString() || 'Update failed');
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
          
          {loading ? (
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
              {deviceStatus.needsFirmwareUpdate ? (
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
                  <Text fontSize="sm" color="gray.400">
                    Version: {deviceStatus.firmwareCheck?.currentVersion}
                  </Text>
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
  );
} 