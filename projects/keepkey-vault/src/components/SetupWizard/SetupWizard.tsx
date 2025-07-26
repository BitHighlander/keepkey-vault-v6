import {
  Box,
  Button,
  HStack,
  VStack,
  Text,
  Flex,
  Icon,
} from "@chakra-ui/react";
import { useState, useEffect } from "react";
import { FaCheckCircle } from "react-icons/fa";
import { useDialog } from "../../contexts/DialogContext";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";

// Import individual steps
import { Step1ConnectDevice } from "./steps/Step1ConnectDevice";
import { Step2VerifyBootloader } from "./steps/Step2VerifyBootloader";
import { Step3VerifyFirmware } from "./steps/Step3VerifyFirmware";
import { Step4SetupWallet } from "./steps/Step4SetupWallet";

interface SetupWizardProps {
  onClose?: () => void;
  onComplete?: () => void;
  initialDeviceId?: string; // Optional pre-selected device
}

interface Step {
  id: string;
  label: string;
  description: string;
  component: React.ComponentType<StepProps>;
}

interface StepProps {
  onNext: (data?: any) => void;
  onPrevious: () => void;
  onSkip?: () => void;
  deviceId?: string;
  stepData?: any;
}

const STEPS: Step[] = [
  {
    id: "connect-device",
    label: "Connect",
    description: "Connect your KeepKey device",
    component: Step1ConnectDevice,
  },
  {
    id: "verify-bootloader",
    label: "Bootloader",
    description: "Verify and update bootloader if needed",
    component: Step2VerifyBootloader,
  },
  {
    id: "verify-firmware",
    label: "Firmware",
    description: "Verify and update firmware if needed",
    component: Step3VerifyFirmware,
  },
  {
    id: "setup-wallet",
    label: "Wallet",
    description: "Initialize your wallet",
    component: Step4SetupWallet,
  },
];

export function SetupWizard({ onClose, onComplete, initialDeviceId }: SetupWizardProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [deviceId, setDeviceId] = useState<string | undefined>(initialDeviceId);
  const [stepData, setStepData] = useState<any>({});
  const [loading, setLoading] = useState(false);
  const highlightColor = "blue.500";
  const { hide } = useDialog();

  // Set setup in progress flag when component mounts
  useEffect(() => {
    console.log('🛡️ SetupWizard: Setting KEEPKEY_SETUP_IN_PROGRESS = true');
    (window as any).KEEPKEY_SETUP_IN_PROGRESS = true;
    
    // Cleanup when component unmounts
    return () => {
      console.log('🛡️ SetupWizard: Clearing KEEPKEY_SETUP_IN_PROGRESS flag');
      (window as any).KEEPKEY_SETUP_IN_PROGRESS = false;
    };
  }, []);

  // Load existing setup progress when component mounts
  useEffect(() => {
    if (initialDeviceId) {
      loadSetupProgress(initialDeviceId);
    }
  }, [initialDeviceId]);

  const loadSetupProgress = async (deviceId: string) => {
    try {
      setLoading(true);
      const device = await invoke<any>('get_device_from_registry', { deviceId });
      
      if (device) {
        const lastStep = device.setup_step_completed || 0;
        console.log(`📋 Loading setup progress for ${deviceId}: step ${lastStep}`);
        
        // If device has some progress, start from the next incomplete step
        if (lastStep > 0 && lastStep < STEPS.length) {
          setCurrentStep(lastStep);
        }
        
        // Set device ID if we have setup progress
        setDeviceId(deviceId);
      }
    } catch (error) {
      console.error('Failed to load setup progress:', error);
    } finally {
      setLoading(false);
    }
  };

  const updateStepProgress = async (deviceId: string, stepIndex: number) => {
    try {
      await invoke('update_device_setup_step', { 
        deviceId, 
        step: stepIndex + 1 // Database uses 1-based indexing
      });
      console.log(`✅ Updated setup progress for ${deviceId}: step ${stepIndex + 1}`);
    } catch (error) {
      console.error('Failed to update setup progress:', error);
    }
  };

  const handleNext = async (data?: any) => {
    // Store any data from the current step
    if (data) {
      setStepData(prev => ({ ...prev, [STEPS[currentStep].id]: data }));
      
      // Special handling for device connection step
      if (currentStep === 0 && data.deviceId) {
        const newDeviceId = data.deviceId;
        setDeviceId(newDeviceId);
        
        // Register device in the registry
        try {
          const serialNumber = data.device?.serial_number || null;
          const features = data.device?.features ? JSON.stringify(data.device.features) : null;
          
          await invoke('register_device', {
            deviceId: newDeviceId,
            serialNumber,
            features
          });
          console.log(`📝 Registered device in registry: ${newDeviceId}`);
        } catch (error) {
          console.error('Failed to register device:', error);
        }
      }
    }
    
    // Update step progress in database if we have a device ID
    if (deviceId) {
      await updateStepProgress(deviceId, currentStep);
    }
    
    if (currentStep < STEPS.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      await handleComplete();
    }
  };

  const handlePrevious = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleSkip = () => {
    // Some steps can be skipped
    handleNext({ skipped: true });
  };

  const handleComplete = async () => {
    console.log("=== Setup Wizard completed ===");
    
    if (deviceId) {
      try {
        // Get ETH address if the device is set up
        let ethAddress: string | null = null;
        try {
          ethAddress = await invoke<string | null>('get_device_eth_address', { deviceId });
        } catch (error) {
          console.warn('Could not get ETH address:', error);
        }
        
        // Mark setup as complete in database
        await invoke('mark_device_setup_complete', { 
          deviceId, 
          ethAddress 
        });
        console.log(`🎉 Marked setup complete for ${deviceId} (ETH: ${ethAddress})`);
        
        // Emit setup completion event
        try {
          await emit('setup:completed', { 
            device_id: deviceId,
            eth_address: ethAddress 
          });
          console.log(`📡 Emitted setup completion event for ${deviceId}`);
        } catch (error) {
          console.error('Failed to emit setup completion event:', error);
        }
      } catch (error) {
        console.error('Failed to mark setup complete:', error);
      }
    }
    
    try {
      // Call the completion callback if provided
      if (onComplete) {
        console.log("Calling onComplete callback");
        onComplete();
      }

      // Use multiple methods to ensure the dialog closes
      if (onClose) {
        console.log("Calling onClose callback");
        onClose();
      }

      // Use the dialog context directly to force close after a short delay
      setTimeout(() => {
        hide('setup-wizard');
        console.log('Forced setup wizard closure via DialogContext');
      }, 100);
    } catch (error) {
      console.error("Failed to complete setup wizard:", error);
    }
  };

  // Handle device change (if user goes back and selects different device)
  const handleDeviceChange = (newDeviceId: string) => {
    setDeviceId(newDeviceId);
    setCurrentStep(0); // Reset to first step
    setStepData({}); // Clear step data
    loadSetupProgress(newDeviceId); // Load progress for new device
  };

  const StepComponent = STEPS[currentStep].component;
  const progress = ((currentStep + 1) / STEPS.length) * 100;

  if (loading) {
    return (
      <Box
        w="100%"
        maxW="1400px"
        bg="gray.800"
        borderRadius="xl"
        boxShadow="xl"
        borderWidth="1px"
        borderColor="gray.700"
        p={8}
        display="flex"
        alignItems="center"
        justifyContent="center"
      >
        <VStack gap={4}>
          <Text fontSize="lg" color="white">Loading setup progress...</Text>
        </VStack>
      </Box>
    );
  }

  return (
    <Box
      w="100%"
      maxW="1400px"
      bg="gray.800"
      borderRadius="xl"
      boxShadow="xl"
      borderWidth="1px"
      borderColor="gray.700"
      overflow="hidden"
    >
        {/* Header */}
        <Box 
          p={6} 
          borderBottomWidth="1px" 
          borderColor="gray.700"
          bg="gray.850"
        >
          <VStack gap={4}>
            <Text fontSize="2xl" fontWeight="bold" color={highlightColor}>
              KeepKey Device Setup
            </Text>
            <Text fontSize="md" color="gray.400">
              {STEPS[currentStep].description}
            </Text>
          </VStack>
        </Box>

        {/* Progress */}
        <Box px={6} py={2}>
          <Box 
            h="4px" 
            bg="gray.700" 
            borderRadius="full"
            overflow="hidden"
          >
            <Box 
              h="100%" 
              bg="blue.500" 
              borderRadius="full"
              transition="width 0.3s"
              w={`${progress}%`}
            />
          </Box>
        </Box>

        {/* Step indicators */}
        <HStack gap={4} justify="center" p={4} flexWrap="wrap">
          {STEPS.map((step, index) => (
            <Flex key={step.id} align="center" minW="0">
              <Box
                w={8}
                h={8}
                borderRadius="full"
                bg={index <= currentStep ? highlightColor : "gray.600"}
                display="flex"
                alignItems="center"
                justifyContent="center"
                transition="all 0.3s"
                flexShrink={0}
              >
                {index < currentStep ? (
                  <Icon as={FaCheckCircle} color="white" boxSize={4} />
                ) : (
                  <Text color="white" fontSize="sm" fontWeight="bold">
                    {index + 1}
                  </Text>
                )}
              </Box>
              <Text
                ml={2}
                fontSize="sm"
                fontWeight={index === currentStep ? "bold" : "normal"}
                color={index <= currentStep ? highlightColor : "gray.400"}
              >
                {step.label}
              </Text>
              {index < STEPS.length - 1 && (
                <Box
                  w={6}
                  h={0.5}
                  bg={index < currentStep ? highlightColor : "gray.600"}
                  ml={2}
                  display={{ base: "none", md: "block" }}
                />
              )}
            </Flex>
          ))}
        </HStack>

        {/* Content */}
        <Box
          p={6}
          minH="400px"
          maxH="70vh"
          overflowY="auto"
          bg="gray.800"
          display="flex"
          alignItems="center"
          justifyContent="center"
        >
          <StepComponent 
            onNext={handleNext}
            onPrevious={handlePrevious}
            onSkip={handleSkip}
            deviceId={deviceId}
            stepData={stepData[STEPS[currentStep].id]}
          />
        </Box>

        {/* Footer */}
        <Box p={6} borderTopWidth="1px" borderColor="gray.700" bg="gray.850">
          <HStack justify="space-between">
            <Text fontSize="sm" color="gray.400">
              Step {currentStep + 1} of {STEPS.length}
            </Text>
            <HStack gap={4}>
              <Button
                variant="outline"
                onClick={handlePrevious}
                disabled={currentStep === 0}
                borderColor="gray.600"
                color="gray.300"
                _hover={{ bg: "gray.700" }}
              >
                Previous
              </Button>
              {/* The step components will handle their own Next/Skip buttons */}
            </HStack>
          </HStack>
        </Box>
      </Box>
  );
} 