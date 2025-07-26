import { useState, useEffect } from 'react';
import { Box, Flex, Button, Text, HStack, VStack, Heading } from '@chakra-ui/react';
import { FaWallet, FaCog, FaQuestionCircle } from 'react-icons/fa';
import { invoke } from '@tauri-apps/api/core';
import splashBg from '../assets/splash-bg.png';
import { SettingsDialog } from './SettingsDialog';
import { useDialog } from '../contexts/DialogContext';
import { Logo } from './logo/logo';

type ViewType = 'vault' | 'settings' | 'support';

export const VaultInterface = () => {
  const [currentView, setCurrentView] = useState<ViewType>('vault');
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const { hideAll } = useDialog();

  // Clear any stuck dialogs when component mounts
  useEffect(() => {
    console.log('🏦 VaultInterface mounted - clearing any stuck dialogs');
    hideAll();
  }, [hideAll]);

  const handleSettingsClick = () => {
    setIsSettingsOpen(true);
    setCurrentView('settings');
  };

  const handleCloseSettings = () => {
    setIsSettingsOpen(false);
    setCurrentView('vault');
  };

  const handleSupportClick = async () => {
    try {
      await invoke('open_url', { url: 'https://support.keepkey.com' });
    } catch (error) {
      console.error('Failed to open support URL:', error);
      // Fallback to window.open if Tauri command fails
      window.open('https://support.keepkey.com', '_blank');
    }
  };

  const handleVaultClick = () => {
    setCurrentView('vault');
    setIsSettingsOpen(false);
  };

  const renderCurrentView = () => {
    switch (currentView) {
      case 'vault':
        return (
                     <VStack gap={8} align="center" justify="center" height="100%">
             <Logo />
             <VStack gap={4} textAlign="center">
               <Heading size="lg" color="white">
                 KeepKey Vault
               </Heading>
               <Text color="gray.300" maxWidth="400px">
                 Your secure hardware wallet interface. Connect your KeepKey device to get started.
               </Text>
             </VStack>
           </VStack>
        );
      default:
        return renderCurrentView();
    }
  };

  return (
    <Box 
      height="100vh" 
      width="100vw" 
      position="relative"
      backgroundImage={`url(${splashBg})`}
      backgroundSize="cover"
      backgroundPosition="center"
    >
      {/* Main Vault Interface - Hidden when settings is open */}
      {!isSettingsOpen && (
        <Box height="100%" display="flex" flexDirection="column">
          {/* Main Content Area */}
          <Box flex="1" overflow="hidden">
            {renderCurrentView()}
          </Box>

          {/* Bottom Navigation */}
          <Box
            height="80px"
            bg="gray.900"
            borderTop="1px solid"
            borderColor="gray.700"
            px={4}
            py={2}
          >
            <HStack justify="space-around" align="center" height="100%">
              <Button
                variant="ghost"
                size="sm"
                height="60px"
                minWidth="60px"
                flexDirection="column"
                gap={1}
                color={currentView === 'vault' ? "blue.400" : "gray.400"}
                _hover={{
                  color: "blue.300",
                  bg: "gray.800",
                }}
                _active={{
                  bg: "gray.700",
                }}
                onClick={handleVaultClick}
              >
                <Box fontSize="lg"><FaWallet /></Box>
                <Text fontSize="xs" fontWeight="medium">
                  Vault
                </Text>
              </Button>

              <Button
                variant="ghost"
                size="sm"
                height="60px"
                minWidth="60px"
                flexDirection="column"
                gap={1}
                color={currentView === 'settings' ? "blue.400" : "gray.400"}
                _hover={{
                  color: "blue.300",
                  bg: "gray.800",
                }}
                _active={{
                  bg: "gray.700",
                }}
                onClick={handleSettingsClick}
              >
                <Box fontSize="lg"><FaCog /></Box>
                <Text fontSize="xs" fontWeight="medium">
                  Settings
                </Text>
              </Button>

              <Button
                variant="ghost"
                size="sm"
                height="60px"
                minWidth="60px"
                flexDirection="column"
                gap={1}
                color="gray.400"
                _hover={{
                  color: "blue.300",
                  bg: "gray.800",
                }}
                _active={{
                  bg: "gray.700",
                }}
                onClick={handleSupportClick}
              >
                <Box fontSize="lg"><FaQuestionCircle /></Box>
                <Text fontSize="xs" fontWeight="medium">
                  Support
                </Text>
              </Button>
            </HStack>
          </Box>
        </Box>
      )}

      {/* Full-Screen Settings Overlay */}
      {isSettingsOpen && (
        <Box
          position="absolute"
          top="0"
          left="0"
          right="0"
          bottom="0"
          backgroundImage={`url(${splashBg})`}
          backgroundSize="cover"
          backgroundPosition="center"
          zIndex="modal"
          display="flex"
          alignItems="center"
          justifyContent="center"
        >
          <SettingsDialog isOpen={isSettingsOpen} onClose={handleCloseSettings} />
        </Box>
      )}
    </Box>
  );
}; 