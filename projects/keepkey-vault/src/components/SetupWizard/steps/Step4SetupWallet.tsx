import {
  Box,
  Button,
  Card,
  HStack,
  Text,
  VStack,
  Icon,
  Spinner,
} from "@chakra-ui/react";
import { FaWallet, FaCheckCircle, FaExclamationTriangle } from "react-icons/fa";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WalletCreationWizard } from "../../WalletCreationWizard/WalletCreationWizard";

interface StepProps {
  onNext: (data?: any) => void;
  onPrevious: () => void;
  onSkip?: () => void;
  deviceId?: string;
  stepData?: any;
}

interface DeviceFeatures {
  initialized: boolean;
  needsBackup?: boolean;
  label?: string;
  version?: string;
}

export function Step4SetupWallet({ onNext, onSkip, deviceId }: StepProps) {
  const [loading, setLoading] = useState(true);
  const [deviceFeatures, setDeviceFeatures] = useState<DeviceFeatures | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showWalletWizard, setShowWalletWizard] = useState(false);
  const [walletSetupComplete, setWalletSetupComplete] = useState(false);

  useEffect(() => {
    if (deviceId) {
      checkDeviceStatus();
    }
  }, [deviceId]);

  const checkDeviceStatus = async () => {
    if (!deviceId) return;
    
    try {
      setLoading(true);
      setError(null);
      
      // Call backend to get device features and check initialization status
      const features = await invoke<DeviceFeatures>('get_features', { deviceId });
      setDeviceFeatures(features);
      
      // If device is already initialized, mark as complete
      if (features.initialized) {
        setWalletSetupComplete(true);
      }
    } catch (error) {
      console.error('Failed to check device status:', error);
      setError(error?.toString() || 'Failed to check device status');
    } finally {
      setLoading(false);
    }
  };

  const handleStartWalletSetup = () => {
    setShowWalletWizard(true);
  };

  const handleWalletSetupComplete = () => {
    setShowWalletWizard(false);
    setWalletSetupComplete(true);
    // Re-check device status to confirm initialization
    checkDeviceStatus();
  };

  const handleWalletSetupClose = () => {
    setShowWalletWizard(false);
    // Check if device was initialized during the process
    checkDeviceStatus();
  };

  const handleContinue = () => {
    onNext({ 
      walletSetupComplete: true,
      deviceInitialized: deviceFeatures?.initialized,
      deviceLabel: deviceFeatures?.label
    });
  };

  const handleSkip = () => {
    if (onSkip) {
      onSkip();
    } else {
      onNext({ skipped: true, deviceInitialized: deviceFeatures?.initialized });
    }
  };

  // If wallet wizard is open, show it instead of the step content
  if (showWalletWizard && deviceId) {
    return (
      <WalletCreationWizard
        deviceId={deviceId}
        onComplete={handleWalletSetupComplete}
        onClose={handleWalletSetupClose}
      />
    );
  }

  return (
    <Box width="full" maxWidth="2xl">
      <Card.Root bg="gray.900" borderColor="gray.700">
        <Card.Header bg="gray.850">
          <HStack justify="center" gap={3}>
            <Icon asChild color="blue.500">
              <FaWallet />
            </Icon>
            <Text fontSize="xl" fontWeight="bold" color="white">
              Setup Wallet
            </Text>
          </HStack>
        </Card.Header>
        <Card.Body>
          <VStack gap={6}>
            <Text color="gray.400" textAlign="center">
              Initialize your wallet to start using your KeepKey device securely.
            </Text>
            
            {loading ? (
              <VStack gap={4}>
                <Spinner size="lg" color="blue.500" />
                <Text color="gray.400">Checking device initialization status...</Text>
              </VStack>
            ) : error ? (
              <VStack gap={4} p={6} bg="red.900" borderRadius="md" borderWidth="1px" borderColor="red.600">
                <Icon as={FaExclamationTriangle} boxSize={12} color="red.400" />
                <Text fontSize="lg" fontWeight="semibold" color="white">
                  Device Check Failed
                </Text>
                <Text color="red.200" textAlign="center">
                  {error}
                </Text>
                <HStack gap={4}>
                  <Button onClick={checkDeviceStatus} colorScheme="red" variant="outline">
                    Retry
                  </Button>
                  <Button onClick={handleSkip} variant="outline">
                    Skip This Step
                  </Button>
                </HStack>
              </VStack>
            ) : deviceFeatures ? (
              <VStack gap={6} width="full">
                {walletSetupComplete || deviceFeatures.initialized ? (
                  <VStack gap={4} p={6} bg="green.900" borderRadius="md" borderWidth="1px" borderColor="green.600">
                    <Icon as={FaCheckCircle} boxSize={12} color="green.400" />
                    <Text fontSize="lg" fontWeight="semibold" color="white">
                      Wallet is Ready
                    </Text>
                    <Text color="green.200" textAlign="center">
                      Your KeepKey device has been successfully initialized and is ready to use.
                    </Text>
                    {deviceFeatures.label && (
                      <Text fontSize="sm" color="gray.400">
                        Device Label: {deviceFeatures.label}
                      </Text>
                    )}
                    {deviceFeatures.version && (
                      <Text fontSize="sm" color="gray.400">
                        Firmware: {deviceFeatures.version}
                      </Text>
                    )}
                  </VStack>
                ) : (
                  <VStack gap={6} width="full">
                    <Box w="full" p={6} bg="blue.900" borderRadius="md" borderWidth="1px" borderColor="blue.600">
                      <VStack gap={4}>
                        <HStack gap={3}>
                          <Icon as={FaWallet} color="blue.400" boxSize={6} />
                          <Text fontSize="lg" fontWeight="bold" color="white">
                            Wallet Initialization Required
                          </Text>
                        </HStack>
                        
                        <Text color="blue.200" textAlign="center">
                          Your device needs to be initialized with a wallet before you can use it. 
                          This process will set up your device label, PIN, and recovery phrase.
                        </Text>
                        
                        <VStack gap={2} pt={2}>
                          <Text fontSize="sm" fontWeight="bold" color="blue.300">
                            The setup process will guide you through:
                          </Text>
                          <Text fontSize="sm" color="gray.300" textAlign="center">
                            • Setting a device label
                          </Text>
                          <Text fontSize="sm" color="gray.300" textAlign="center">
                            • Creating a secure PIN
                          </Text>
                          <Text fontSize="sm" color="gray.300" textAlign="center">
                            • Generating or importing your recovery phrase
                          </Text>
                          <Text fontSize="sm" color="gray.300" textAlign="center">
                            • Confirming your security settings
                          </Text>
                        </VStack>
                        
                        <Button
                          onClick={handleStartWalletSetup}
                          colorScheme="blue"
                          size="lg"
                          width="full"
                        >
                          Start Wallet Setup
                        </Button>
                      </VStack>
                    </Box>
                    
                    <Button
                      onClick={handleSkip}
                      variant="outline"
                      colorScheme="gray"
                      size="sm"
                    >
                      Skip Wallet Setup (Not Recommended)
                    </Button>
                  </VStack>
                )}
                
                {(walletSetupComplete || deviceFeatures.initialized) && (
                  <Button
                    onClick={handleContinue}
                    colorScheme="green"
                    size="lg"
                    width="full"
                  >
                    Complete Setup
                  </Button>
                )}
              </VStack>
            ) : null}
          </VStack>
        </Card.Body>
      </Card.Root>
    </Box>
  );
} 