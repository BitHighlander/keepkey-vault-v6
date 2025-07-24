// Stub for pinService
import { PinStep } from '../types/pin';

export const PinService = {
  startPinCreation: async (deviceId: string, sessionType: string) => ({
    device_id: deviceId,
    session_id: 'test-session',
    current_step: PinStep.AwaitingFirst,
    is_active: true
  }),
  validatePositions: async (positions: any) => Promise.resolve({
    valid: true,
    error: null
  }),
  sendPinResponse: async (sessionId: string, positions: any) => ({
    success: true,
    next_step: 'confirm',
    error: null
  }),
  getSessionStatus: async (sessionId: string) => ({ 
    device_id: 'mock-device',
    session_id: sessionId,
    current_step: PinStep.AwaitingSecond,
    is_active: true
  })
};

export const pinService = PinService;