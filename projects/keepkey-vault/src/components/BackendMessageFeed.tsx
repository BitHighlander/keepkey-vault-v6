import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Box, Text, VStack, Flex } from '@chakra-ui/react';

interface BackendMessage {
  id: string;
  message: string;
  timestamp: Date;
  type: 'log' | 'device' | 'error';
}

interface BackendMessageFeedProps {
  maxMessages?: number;
  showTimestamp?: boolean;
}

export const BackendMessageFeed = ({ maxMessages = 10, showTimestamp = false }: BackendMessageFeedProps) => {
  const [messages, setMessages] = useState<BackendMessage[]>([]);
  const [latestMessage, setLatestMessage] = useState<string>('Initializing...');

  useEffect(() => {
    let unlistenBackendLog: (() => void) | undefined;
    let unlistenDeviceConnected: (() => void) | undefined;
    let unlistenDeviceDisconnected: (() => void) | undefined;

    const setupListeners = async () => {
      console.log('🎯 Setting up backend message listeners...');

      // Listen for backend log messages
      unlistenBackendLog = await listen('backend:log', (event) => {
        const message = event.payload as string;
        console.log('📱 [BackendMessageFeed] Backend log:', message);
        
        const newMessage: BackendMessage = {
          id: `${Date.now()}-${Math.random()}`,
          message,
          timestamp: new Date(),
          type: 'log'
        };

        setMessages(prev => {
          const updated = [newMessage, ...prev];
          return updated.slice(0, maxMessages);
        });
        
        setLatestMessage(message);
      });

      // Listen for device connection events (these will also be in backend:log but we can handle separately if needed)
      unlistenDeviceConnected = await listen('device:connected', (event) => {
        const deviceId = event.payload as string;
        console.log('📱 [BackendMessageFeed] Device connected:', deviceId);
        // The backend:log event will handle the display, this is just for additional logic if needed
      });

      unlistenDeviceDisconnected = await listen('device:disconnected', (event) => {
        const deviceId = event.payload as string;
        console.log('📱 [BackendMessageFeed] Device disconnected:', deviceId);
        // The backend:log event will handle the display, this is just for additional logic if needed
      });
    };

    setupListeners().catch(console.error);

    return () => {
      unlistenBackendLog?.();
      unlistenDeviceConnected?.();
      unlistenDeviceDisconnected?.();
    };
  }, [maxMessages]);

  return (
    <Box>
      {/* Current/Latest Status */}
      <Text fontSize="lg" fontWeight="medium" color="white" mb={2}>
        {latestMessage}
      </Text>

      {/* Message History (optional, for debugging) */}
      {showTimestamp && (
        <VStack align="stretch" mt={4} opacity={0.7} gap={1}>
          {messages.slice(0, 5).map((msg) => (
            <Flex key={msg.id} justify="space-between" fontSize="xs" color="gray.300">
              <Text>{msg.message}</Text>
              <Text>{msg.timestamp.toLocaleTimeString()}</Text>
            </Flex>
          ))}
        </VStack>
      )}
    </Box>
  );
};

// Hook to get just the latest backend message
export const useLatestBackendMessage = () => {
  const [latestMessage, setLatestMessage] = useState<string>('Waiting for backend...');
  const [isConnected, setIsConnected] = useState<boolean>(false);

  useEffect(() => {
    let unlistenBackendLog: (() => void) | undefined;
    let unlistenDeviceConnected: (() => void) | undefined;
    let unlistenDeviceDisconnected: (() => void) | undefined;

    const setupListeners = async () => {
      console.log('🎯 [useLatestBackendMessage] Setting up backend:log listener...');
      
      // Listen for backend log messages
      unlistenBackendLog = await listen('backend:log', (event) => {
        const message = event.payload as string;
        console.log('🎯 [useLatestBackendMessage] Received backend:log:', message);
        setLatestMessage(message);
      });

      // Track device connection state
      unlistenDeviceConnected = await listen('device:connected', (event) => {
        console.log('🎯 [useLatestBackendMessage] Device connected:', event.payload);
        setIsConnected(true);
      });

      unlistenDeviceDisconnected = await listen('device:disconnected', (event) => {
        console.log('🎯 [useLatestBackendMessage] Device disconnected:', event.payload);
        setIsConnected(false);
      });

      console.log('🎯 [useLatestBackendMessage] All listeners set up');
    };

    setupListeners().catch((error) => {
      console.error('🎯 [useLatestBackendMessage] Failed to setup listeners:', error);
    });

    return () => {
      console.log('🎯 [useLatestBackendMessage] Cleaning up listeners');
      unlistenBackendLog?.();
      unlistenDeviceConnected?.();
      unlistenDeviceDisconnected?.();
    };
  }, []);

  return { latestMessage, isConnected };
}; 