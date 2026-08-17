import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ConfigView from './components/ConfigView';
import RunningView from './components/RunningView';
import { SpooferConfig, SpooferState } from './types';

function App() {
  const [isRunning, setIsRunning] = useState(false);
  const [spooferState, setSpooferState] = useState<SpooferState | null>(null);

  useEffect(() => {
    // Listen for state updates from backend
    const unlisten = listen<SpooferState>('spoofer-state', (event) => {
      setSpooferState(event.payload);

      // Update running status
      const status = event.payload.status;
      const running = status === 'Running' || status === 'Starting';
      setIsRunning(running);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleStart = async (config: SpooferConfig) => {
    try {
      await invoke('start_spoofing', { config });
      setIsRunning(true);
    } catch (error) {
      console.error('Failed to start spoofing:', error);
      alert(`Failed to start: ${error}`);
    }
  };

  const handleStop = async () => {
    try {
      await invoke('stop_spoofing');
      setIsRunning(false);
      setSpooferState(null);
    } catch (error) {
      console.error('Failed to stop spoofing:', error);
      alert(`Failed to stop: ${error}`);
    }
  };

  return (
    <div className="app">
      <div className="container">
        <div className="header">
          <div className="logo">
            <div className="logo-icon">
              <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M13 3L4 14h7l-1 7 9-11h-7l1-7z" fill="currentColor" />
              </svg>
            </div>
            <h1>Pierro Spoof</h1>
          </div>
          <p className="subtitle">Ratio Enhancement Tool</p>
        </div>
        {!isRunning ? (
          <ConfigView onStart={handleStart} />
        ) : (
          <RunningView state={spooferState} onStop={handleStop} />
        )}
      </div>
    </div>
  );
}

export default App;
