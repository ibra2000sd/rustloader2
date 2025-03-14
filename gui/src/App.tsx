// src/App.tsx
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import DownloadForm from './components/DownloadForm';
import ProgressBar from './components/ProgressBar';
import LicenseInfo from './components/LicenseInfo';
import Header from './components/Header';
import Footer from './components/Footer';
import Tabs from './components/Tabs';
import Alert from './components/Alert';

// Type definitions
export type TabType = 'download' | 'license';
export type LicenseStatus = 'checking' | 'free' | 'pro';

export interface DownloadParams {
  url: string;
  quality: string;
  format: string;
  startTime?: string;
  endTime?: string;
  usePlaylist: boolean;
  downloadSubtitles: boolean;
  outputDir?: string;
}

const App: React.FC = () => {
  // Application state
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus>('checking');
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [activeTab, setActiveTab] = useState<TabType>('download');
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [downloadInfo, setDownloadInfo] = useState<{
    fileName?: string;
    fileSize?: number;
    speed?: number;
    timeRemaining?: number;
  }>({});

  // Check license status on mount
  useEffect(() => {
    const checkLicense = async () => {
      try {
        const isPro = await invoke<boolean>('is_pro');
        setLicenseStatus(isPro ? 'pro' : 'free');
      } catch (err) {
        console.error('Failed to check license:', err);
        setLicenseStatus('free');
        setError('License check failed. Using free version.');
      }
    };

    checkLicense();
  }, []);

  // Check for pending downloads
  useEffect(() => {
    const checkPendingDownloads = async () => {
      try {
        const progress = await invoke<number>('get_progress');
        if (progress > 0) {
          setIsDownloading(true);
          setDownloadProgress(progress);
        }
      } catch (err) {
        console.error('Failed to check pending downloads:', err);
      }
    };

    checkPendingDownloads();
  }, []);

  // Listen for download progress events
  useEffect(() => {
    const setupListener = async () => {
      try {
        const unlisten = await listen<{
          progress: number;
          fileName?: string;
          fileSize?: number;
          speed?: number;
          timeRemaining?: number;
        }>('download-progress', (event) => {
          const { progress, ...info } = event.payload;
          setDownloadProgress(progress);
          setDownloadInfo(info);

          // When download completes
          if (progress >= 100) {
            setTimeout(() => {
              setIsDownloading(false);
              setSuccess('Download completed successfully!');
            }, 2000);
          }
        });

        return unlisten;
      } catch (err) {
        console.error('Failed to set up event listener:', err);
        return () => {};
      }
    };

    const unsubscribe = setupListener();
    return () => {
      unsubscribe.then(unlisten => {
        unlisten();
      }).catch(err => console.error('Failed to unsubscribe:', err));
    };
  }, []);

  // Handle download start
  const handleDownloadStart = async (params: DownloadParams) => {
    setError('');
    setSuccess('');

    try {
      setIsDownloading(true);
      setDownloadProgress(0);

      await invoke('start_download', {
        url: params.url,
        quality: params.quality || undefined,
        format: params.format,
        startTime: params.startTime || undefined,
        endTime: params.endTime || undefined,
        usePlaylist: params.usePlaylist,
        downloadSubtitles: params.downloadSubtitles,
        outputDir: params.outputDir || undefined,
        progressState: {} // This is handled by the backend
      });
    } catch (err) {
      console.error('Download failed:', err);
      setIsDownloading(false);
      setError(`Download failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  // Handle license activation
  const handleLicenseActivation = async (success: boolean) => {
    if (success) {
      setLicenseStatus('pro');
      setSuccess('License activated successfully!');
    }
  };

  return (
    <div className="h-screen w-screen overflow-hidden bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      <div className="flex flex-col h-full w-full max-w-app mx-auto">
        <Header licenseStatus={licenseStatus} />
        
        <main className="flex-1 overflow-auto p-4">
          {/* Alerts */}
          {error && <Alert type="error" message={error} onDismiss={() => setError('')} />}
          {success && <Alert type="success" message={success} onDismiss={() => setSuccess('')} />}
          
          {/* Tabs */}
          <Tabs activeTab={activeTab} onChange={setActiveTab} />
          
          {/* Progress Bar (shown during download) */}
          {isDownloading && (
            <div className="mb-4">
              <ProgressBar 
                progress={downloadProgress} 
                fileName={downloadInfo.fileName}
                fileSize={downloadInfo.fileSize}
                speed={downloadInfo.speed}
                timeRemaining={downloadInfo.timeRemaining}
              />
            </div>
          )}
          
          {/* Tab Content */}
          <div className="mt-4">
            {activeTab === 'download' ? (
              <DownloadForm 
                isPro={licenseStatus === 'pro'} 
                onDownloadStart={handleDownloadStart}
                disabled={isDownloading}
              />
            ) : (
              <LicenseInfo 
                isProVersion={licenseStatus === 'pro'} 
                onActivationComplete={handleLicenseActivation}
              />
            )}
          </div>
        </main>
        
        <Footer />
      </div>
    </div>
  );
};

export default App;