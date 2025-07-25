import {
  Box,
  Button,
  Card,
  Text,
  VStack,
  Code,
} from "@chakra-ui/react";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface StepProps {
  onNext: (data?: any) => void;
  onPrevious: () => void;
  onSkip?: () => void;
  deviceId?: string;
  stepData?: any;
}

export function Step2VerifyBootloader({ onNext, deviceId }: StepProps) {
  const [features, setFeatures] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (deviceId) {
      getDeviceFeatures();
    }
  }, [deviceId]);

  const getDeviceFeatures = async () => {
    try {
      setLoading(true);
      const result = await invoke('get_features', { deviceId });
      console.log('Raw device features:', result);
      setFeatures(result);
    } catch (err) {
      console.error('Failed to get features:', err);
      setError(err?.toString() || 'Failed to get device features');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box maxWidth="800px" margin="auto">
      <Card.Root size="lg">
        <Card.Header>
          <Text fontSize="xl" fontWeight="bold">Raw Device Features Debug</Text>
        </Card.Header>

        <Card.Body>
          <VStack gap={4} align="stretch">
            {loading && <Text>Loading device features...</Text>}
            
            {error && (
              <Box p={4} bg="red.900" borderRadius="md">
                <Text color="red.300">Error: {error}</Text>
              </Box>
            )}

            {features && (
              <VStack gap={4} align="stretch">
                {/* Bootloader Info Summary */}
                <Box p={4} bg="blue.900" borderRadius="md" borderWidth="1px" borderColor="blue.500">
                  <Text fontSize="lg" fontWeight="bold" color="blue.200" mb={2}>
                    🔍 Bootloader Analysis
                  </Text>
                  <VStack gap={2} align="stretch">
                    <Box>
                      <Text fontSize="sm" color="gray.400">Bootloader Hash:</Text>
                      <Code colorScheme="orange" fontSize="xs">
                        {features.bootloader_hash || 'None'}
                      </Code>
                    </Box>
                    <Box>
                      <Text fontSize="sm" color="gray.400">Bootloader Version (Mapped):</Text>
                      <Code colorScheme="green" fontSize="sm">
                        {features.bootloader_version || 'None'}
                      </Code>
                    </Box>
                    <Box>
                      <Text fontSize="sm" color="gray.400">Firmware Version:</Text>
                      <Code colorScheme="blue">{features.version}</Code>
                    </Box>
                    <Box>
                      <Text fontSize="sm" color="gray.400">Bootloader Mode:</Text>
                      <Code colorScheme={features.bootloader_mode ? "red" : "green"}>
                        {features.bootloader_mode ? "YES (Update Mode)" : "NO (Normal Mode)"}
                      </Code>
                    </Box>
                  </VStack>
                </Box>

                {/* Raw JSON */}
                <Box>
                  <Text mb={2} fontWeight="bold">Device Features (Raw JSON):</Text>
                  <Code 
                    display="block" 
                    whiteSpace="pre-wrap" 
                    p={4} 
                    bg="gray.900" 
                    borderRadius="md"
                    fontSize="sm"
                    maxHeight="300px"
                    overflowY="auto"
                  >
                    {JSON.stringify(features, null, 2)}
                  </Code>
                </Box>
              </VStack>
            )}
          </VStack>
        </Card.Body>

        <Card.Footer>
          <Button onClick={() => onNext({ debugData: features })}>
            Continue (Debug)
          </Button>
        </Card.Footer>
      </Card.Root>
    </Box>
  );
} 