// Stub for BootloaderUpdateWizard
export const BootloaderUpdateWizard = () => {
  return <div>BootloaderUpdateWizard Stub</div>;
};

export interface BootloaderUpdateWizardProps {
  deviceId?: string;
  currentVersion?: string;
  requiredVersion?: string;
  onClose?: () => void;
  onComplete?: () => void;
}