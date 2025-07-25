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
import { FaShieldAlt, FaCheckCircle, FaExclamationTriangle } from "react-icons/fa";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface StepProps {
  onNext: (data?: any) => void;
  onPrevious: () => void;
  onSkip?: () => void;
  deviceId?: string;
  stepData?: any;
}

interface BootloaderCheck {
  needsUpdate: boolean;
  currentVersion: string;
  latestVersion: string;
  isRequired: boolean;
  severity: 'low' | 'medium' | 'high' | 'critical';
}

export function Step2VerifyBootloader({ onNext, onSkip, deviceId }: StepProps) {
  const [loading, setLoading] = useState(true);
  const [bootloaderCheck, setBootloaderCheck] = useState<BootloaderCheck | null>(null);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [updateComplete, setUpdateComplete] = useState(false);

  useEffect(() => {
    if (deviceId) {
      checkBootloader();
    }
  }, [deviceId]);

  const checkBootloader = async () => {
    if (!deviceId) return;
    
    try {
      setLoading(true);
      setError(null);
      
      // Call backend to check bootloader status
      const result = await invoke<BootloaderCheck>('check_device_bootloader', { deviceId });
      setBootloaderCheck(result);
    } catch (error) {
      console.error('Failed to check bootloader:', error);
      setError(error?.toString() || 'Failed to check bootloader status');
    } finally {
      setLoading(false);
    }
  };

  const handleUpdate = async () => {
    if (!deviceId || !bootloaderCheck) return;
    
    try {
      setUpdating(true);
      setError(null);
      
      // Call backend to update bootloader
      const success = await invoke<boolean>('update_device_bootloader', {
        deviceId,
        targetVersion: bootloaderCheck.latestVersion
      });
      
      if (success) {
        setUpdateComplete(true);
        // Re-check bootloader status after update
        setTimeout(() => {
          checkBootloader();
        }, 2000);
      }
    } catch (error) {
      console.error('Bootloader update failed:', error);
      setError(error?.toString() || 'Update failed');
    } finally {
      setUpdating(false);
    }
  };

  const handleContinue = () => {
    onNext({ 
      bootloaderChecked: true,
      bootloaderUpdated: updateComplete,
      bootloaderStatus: bootloaderCheck
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
              <FaShieldAlt />
            </Icon>
            <Text fontSize="xl" fontWeight="bold" color="white">
              Verify Bootloader
            </Text>
          </HStack>
        </Card.Header>
        <Card.Body>
          <VStack gap={6}>
            <Text color="gray.400" textAlign="center">
              Checking your device's bootloader for security and compatibility.
            </Text>
            
            {loading ? (
              <VStack gap={4}>
                <Spinner size="lg" color="blue.500" />
                <Text color="gray.400">Checking bootloader status...</Text>
              </VStack>
            ) : error ? (
              <VStack gap={4} p={6} bg="red.900" borderRadius="md" borderWidth="1px" borderColor="red.600">
                <Icon as={FaExclamationTriangle} boxSize={12} color="red.400" />
                <Text fontSize="lg" fontWeight="semibold" color="white">
                  Bootloader Check Failed
                </Text>
                <Text color="red.200" textAlign="center">
                  {error}
                </Text>
                <HStack gap={4}>
                  <Button onClick={checkBootloader} colorScheme="red" variant="outline">
                    Retry
                  </Button>
                  <Button onClick={handleSkip} variant="outline">
                    Skip This Step
                  </Button>
                </HStack>
              </VStack>
            ) : bootloaderCheck ? (
              <VStack gap={6} width="full">
                {bootloaderCheck.needsUpdate ? (
                  <Box w="full" p={6} bg="orange.900" borderRadius="md" borderWidth="1px" borderColor="orange.600">
                    <VStack gap={4}>
                      <HStack gap={3}>
                        <Icon as={FaExclamationTriangle} color="orange.400" boxSize={6} />
                        <Text fontSize="lg" fontWeight="bold" color="white">
                          Bootloader Update Available
                        </Text>
                        <Badge colorScheme={getSeverityColor(bootloaderCheck.severity)}>
                          {getSeverityLabel(bootloaderCheck.severity)}
                        </Badge>
                      </HStack>
                      
                      <HStack gap={8} width="full" justify="space-around">
                        <VStack>
                          <Text fontSize="sm" color="gray.400">Current Version</Text>
                          <Text fontSize="lg" fontWeight="semibold">{bootloaderCheck.currentVersion}</Text>
                        </VStack>
                        <Icon as={FaShieldAlt} color="orange.400" boxSize={8} />
                        <VStack>
                          <Text fontSize="sm" color="gray.400">Latest Version</Text>
                          <Text fontSize="lg" fontWeight="semibold" color="green.400">
                            {bootloaderCheck.latestVersion}
                          </Text>
                        </VStack>
                      </HStack>
                      
                      <Text color="orange.200" textAlign="center" fontSize="sm">
                        {bootloaderCheck.isRequired 
                          ? "This update is required for security and compatibility."
                          : "This update is recommended for improved security and features."
                        }
                      </Text>
                      
                      {updating ? (
                        <VStack gap={2}>
                          <Spinner color="orange.400" />
                          <Text color="orange.200">Updating bootloader...</Text>
                        </VStack>
                      ) : updateComplete ? (
                        <VStack gap={2}>
                          <Icon as={FaCheckCircle} color="green.400" boxSize={6} />
                          <Text color="green.200">Bootloader updated successfully!</Text>
                        </VStack>
                      ) : (
                        <HStack gap={4}>
                          {!bootloaderCheck.isRequired && (
                            <Button onClick={handleSkip} variant="outline" colorScheme="gray">
                              Skip Update
                            </Button>
                          )}
                          <Button
                            onClick={handleUpdate}
                            colorScheme="orange"
                            size="lg"
                            loading={updating}
                            loadingText="Updating..."
                          >
                            Update Bootloader
                          </Button>
                        </HStack>
                      )}
                    </VStack>
                  </Box>
                ) : (
                  <VStack gap={4} p={6} bg="green.900" borderRadius="md" borderWidth="1px" borderColor="green.600">
                    <Icon as={FaCheckCircle} boxSize={12} color="green.400" />
                    <Text fontSize="lg" fontWeight="semibold" color="white">
                      Bootloader is Up to Date
                    </Text>
                    <Text color="green.200" textAlign="center">
                      Your device's bootloader is current and secure.
                    </Text>
                    <Text fontSize="sm" color="gray.400">
                      Version: {bootloaderCheck.currentVersion}
                    </Text>
                  </VStack>
                )}
                
                <Button
                  onClick={handleContinue}
                  colorScheme="blue"
                  size="lg"
                  width="full"
                  disabled={bootloaderCheck.needsUpdate && bootloaderCheck.isRequired && !updateComplete}
                >
                  Continue to Firmware Check
                </Button>
              </VStack>
            ) : null}
          </VStack>
        </Card.Body>
      </Card.Root>
    </Box>
  );
} 