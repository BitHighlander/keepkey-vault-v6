import {
  Box,
  Button,
  Card,
  HStack,
  Text,
  VStack,
  Icon,
  Spinner,
  Alert,
} from "@chakra-ui/react";
import { FaUsb, FaCheckCircle } from "react-icons/fa";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface StepProps {
  onNext: (data?: any) => void;
  onPrevious: () => void;
  onSkip?: () => void;
  deviceId?: string;
  stepData?: any;
}

interface Device {
  id: string;
  name: string;
  manufacturer?: string;
  features?: any;
}

export function Step1ConnectDevice({ onNext, deviceId: preselectedDeviceId }: StepProps) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(null);
  const [loading, setLoading] = useState(true);
  const [connectStatus, setConnectStatus] = useState<string | null>(null);

  useEffect(() => {
    loadDevices();
    
    // Listen for device connection events
    const unlistenConnected = listen('device:connected', (event: any) => {
      console.log('Device connected:', event.payload);
      loadDevices();
    });
    
    const unlistenDisconnected = listen('device:disconnected', (event: any) => {
      console.log('Device disconnected:', event.payload);
      loadDevices();
    });
    
    // Listen for feature retry events
    const unlistenRetrying = listen('feature:retrying', (event: any) => {
      const { attempt, max } = event.payload || {};
      if (attempt && max) {
        if (attempt < max) {
          setConnectStatus(`Connecting to device (${attempt}/${max})...`);
        } else {
          setConnectStatus(`Connection failed after ${max} attempts. Please reconnect your KeepKey.`);
        }
      }
    });

    return () => {
      unlistenConnected.then((fn: any) => fn());
      unlistenDisconnected.then((fn: any) => fn());
      unlistenRetrying.then((fn: any) => fn());
    };
  }, []);

  const loadDevices = async () => {
    try {
      setLoading(true);
      const connectedDevices = await invoke<any[]>('get_connected_devices');
      
      const deviceList: Device[] = connectedDevices.map(device => ({
        id: device.device_id,
        name: device.name || 'KeepKey',
        manufacturer: 'KeepKey',
        features: device.features
      }));
      
      setDevices(deviceList);
      
      // If there's a preselected device, try to find it
      if (preselectedDeviceId) {
        const preselected = deviceList.find(d => d.id === preselectedDeviceId);
        if (preselected) {
          setSelectedDevice(preselected);
        }
      }
      
      setConnectStatus(null);
    } catch (error) {
      console.error('Failed to load devices:', error);
      setConnectStatus('Failed to scan for devices. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const handleDeviceSelect = (device: Device) => {
    setSelectedDevice(device);
  };

  const handleContinue = () => {
    if (selectedDevice) {
      onNext({ deviceId: selectedDevice.id, device: selectedDevice });
    }
  };

  const handleRefresh = () => {
    loadDevices();
  };

  return (
    <Box width="full" maxWidth="2xl">
      <Card.Root bg="gray.900" borderColor="gray.700">
        <Card.Header bg="gray.850">
          <HStack justify="center" gap={3}>
            <Icon asChild color="blue.500">
              <FaUsb />
            </Icon>
            <Text fontSize="xl" fontWeight="bold" color="white">
              Connect Your KeepKey Device
            </Text>
          </HStack>
        </Card.Header>
        <Card.Body>
          <VStack gap={6}>
            <Text color="gray.400" textAlign="center">
              Connect your KeepKey device via USB to begin the setup process.
            </Text>
            
            {connectStatus && (
              <Alert.Root status={connectStatus.includes('failed') ? 'error' : 'info'}>
                <Alert.Title>{connectStatus}</Alert.Title>
              </Alert.Root>
            )}
            
            {loading ? (
              <VStack gap={4}>
                <Spinner size="lg" color="blue.500" />
                <Text color="gray.400">Scanning for KeepKey devices...</Text>
              </VStack>
            ) : devices.length === 0 ? (
              <VStack gap={4} p={6} bg="gray.800" borderRadius="md" borderWidth="1px" borderColor="gray.600">
                <Icon as={FaUsb} boxSize={12} color="gray.500" />
                <Text fontSize="lg" fontWeight="semibold" color="white">
                  No KeepKey Device Found
                </Text>
                <Text color="gray.400" textAlign="center">
                  Please connect your KeepKey device via USB and ensure it's properly connected.
                </Text>
                <Button onClick={handleRefresh} colorScheme="blue" variant="outline">
                  Refresh
                </Button>
              </VStack>
            ) : (
              <VStack gap={4} width="full">
                <Text fontSize="lg" fontWeight="semibold" color="white">
                  Select Your Device
                </Text>
                
                {devices.map((device) => (
                  <Box
                    key={device.id}
                    w="full"
                    p={4}
                    bg={selectedDevice?.id === device.id ? "blue.900" : "gray.800"}
                    borderRadius="md"
                    borderWidth="2px"
                    borderColor={selectedDevice?.id === device.id ? "blue.500" : "gray.600"}
                    cursor="pointer"
                    onClick={() => handleDeviceSelect(device)}
                    transition="all 0.2s"
                    _hover={{ borderColor: "blue.400", bg: "blue.950" }}
                  >
                    <HStack justify="space-between">
                      <HStack gap={3}>
                        <Icon as={FaUsb} color="blue.400" boxSize={5} />
                        <VStack align="start" gap={1}>
                          <Text fontWeight="semibold" color="white">
                            {device.name}
                          </Text>
                          <Text fontSize="sm" color="gray.400">
                            Device ID: {device.id.slice(0, 8)}...
                          </Text>
                        </VStack>
                      </HStack>
                      {selectedDevice?.id === device.id && (
                        <Icon as={FaCheckCircle} color="blue.400" boxSize={5} />
                      )}
                    </HStack>
                  </Box>
                ))}
                
                <HStack gap={4} pt={4}>
                  <Button onClick={handleRefresh} variant="outline" colorScheme="gray">
                    Refresh
                  </Button>
                  <Button
                    onClick={handleContinue}
                    colorScheme="blue"
                    disabled={!selectedDevice}
                    size="lg"
                    flex="1"
                  >
                    Continue with Selected Device
                  </Button>
                </HStack>
              </VStack>
            )}
          </VStack>
        </Card.Body>
      </Card.Root>
    </Box>
  );
} 