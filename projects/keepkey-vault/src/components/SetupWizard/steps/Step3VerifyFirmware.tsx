import {
  Box,
  Button,
  Card,
  HStack,
  Text,
  VStack,
  Icon,
  Spinner,
  Badge,
} from "@chakra-ui/react";
import { FaDownload, FaCheckCircle, FaExclamationTriangle } from "react-icons/fa";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface StepProps {
  onNext: (data?: any) => void;
  onPrevious: () => void;
  onSkip?: () => void;
  deviceId?: string;
  stepData?: any;
}

interface FirmwareCheck {
  needsUpdate: boolean;
  currentVersion: string;
  latestVersion: string;
  isRequired: boolean;
  severity: 'low' | 'medium' | 'high' | 'critical';
  releaseNotes?: string;
}

export function Step3VerifyFirmware({ onNext, onSkip, deviceId }: StepProps) {
  const [loading, setLoading] = useState(true);
  const [firmwareCheck, setFirmwareCheck] = useState<FirmwareCheck | null>(null);
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
      
      // Call backend to check firmware status
      const result = await invoke<FirmwareCheck>('check_device_firmware', { deviceId });
      setFirmwareCheck(result);
    } catch (error) {
      console.error('Failed to check firmware:', error);
      setError(error?.toString() || 'Failed to check firmware status');
    } finally {
      setLoading(false);
    }
  };

  const handleUpdate = async () => {
    if (!deviceId || !firmwareCheck) return;
    
    try {
      setUpdating(true);
      setError(null);
      
      // Call backend to update firmware
      const success = await invoke<boolean>('update_device_firmware', {
        deviceId,
        targetVersion: firmwareCheck.latestVersion
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
      firmwareStatus: firmwareCheck
    });
  };

  const handleSkip = () => {
    if (onSkip) {
      onSkip();
    } else {
      onNext({ skipped: true });
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical': return 'red';
      case 'high': return 'orange';
      case 'medium': return 'yellow';
      case 'low': return 'blue';
      default: return 'gray';
    }
  };

  const getSeverityLabel = (severity: string) => {
    switch (severity) {
      case 'critical': return 'Critical Update';
      case 'high': return 'Important Update';
      case 'medium': return 'Recommended Update';
      case 'low': return 'Optional Update';
      default: return 'Update Available';
    }
  };

  return (
    <Box width="full" maxWidth="2xl">
      <Card.Root bg="gray.900" borderColor="gray.700">
        <Card.Header bg="gray.850">
          <HStack justify="center" gap={3}>
            <Icon asChild color="blue.500">
              <FaDownload />
            </Icon>
            <Text fontSize="xl" fontWeight="bold" color="white">
              Verify Firmware
            </Text>
          </HStack>
        </Card.Header>
        <Card.Body>
          <VStack gap={6}>
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
            ) : firmwareCheck ? (
              <VStack gap={6} width="full">
                {firmwareCheck.needsUpdate ? (
                  <Box w="full" p={6} bg="blue.900" borderRadius="md" borderWidth="1px" borderColor="blue.600">
                    <VStack gap={4}>
                      <HStack gap={3}>
                        <Icon as={FaDownload} color="blue.400" boxSize={6} />
                        <Text fontSize="lg" fontWeight="bold" color="white">
                          Firmware Update Available
                        </Text>
                        <Badge colorScheme={getSeverityColor(firmwareCheck.severity)}>
                          {getSeverityLabel(firmwareCheck.severity)}
                        </Badge>
                      </HStack>
                      
                      <HStack gap={8} width="full" justify="space-around">
                        <VStack>
                          <Text fontSize="sm" color="gray.400">Current Version</Text>
                          <Text fontSize="lg" fontWeight="semibold">{firmwareCheck.currentVersion}</Text>
                        </VStack>
                        <Icon as={FaDownload} color="blue.400" boxSize={8} />
                        <VStack>
                          <Text fontSize="sm" color="gray.400">Latest Version</Text>
                          <Text fontSize="lg" fontWeight="semibold" color="green.400">
                            {firmwareCheck.latestVersion}
                          </Text>
                        </VStack>
                      </HStack>
                      
                      <Text color="blue.200" textAlign="center" fontSize="sm">
                        {firmwareCheck.isRequired 
                          ? "This update is required for proper device operation."
                          : "This update includes new features and security improvements."
                        }
                      </Text>
                      
                      {firmwareCheck.releaseNotes && (
                        <Box p={3} bg="gray.800" borderRadius="md" borderWidth="1px" borderColor="gray.600" width="full">
                          <Text fontSize="sm" fontWeight="semibold" color="blue.300" mb={2}>
                            What's New:
                          </Text>
                          <Text fontSize="sm" color="gray.300">
                            {firmwareCheck.releaseNotes}
                          </Text>
                        </Box>
                      )}
                      
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
                            {!firmwareCheck.isRequired && (
                              <Button onClick={handleSkip} variant="outline" colorScheme="gray">
                                Skip Update
                              </Button>
                            )}
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
                      Version: {firmwareCheck.currentVersion}
                    </Text>
                  </VStack>
                )}
                
                <Button
                  onClick={handleContinue}
                  colorScheme="blue"
                  size="lg"
                  width="full"
                  disabled={firmwareCheck.needsUpdate && firmwareCheck.isRequired && !updateComplete}
                >
                  Continue to Wallet Setup
                </Button>
              </VStack>
            ) : null}
          </VStack>
        </Card.Body>
      </Card.Root>
    </Box>
  );
} 