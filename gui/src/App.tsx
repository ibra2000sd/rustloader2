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

  useEffect(() => {
    let unlistenProgress: Promise<() => void>;
    let unlistenCompletion: Promise<() => void>;

    const setupListeners = async () => {
      try {
        unlistenProgress = listen<{
          progress: number;
          fileName?: string;
          fileSize?: number;
          speed?: number;
          timeRemaining?: number;
        }>('download-progress', (event) => {
          const { progress, ...info } = event.payload;
          setDownloadProgress(progress);
          setDownloadInfo(info);
        });

        unlistenCompletion = listen<{
          success: boolean;
          message: string;
        }>('download-completed', (event) => {
          const { success, message } = event.payload;
          setTimeout(() => {
            setIsDownloading(false);
            if (success) {
              setSuccess(message || 'Download completed successfully!');
            } else {
              setError(`Download failed: ${message}`);
            }
          }, 1000);
        });
      } catch (err) {
        console.error('Failed to set up event listeners:', err);
      }
    };

    setupListeners();

    return () => {
      unlistenProgress?.then(unlisten => unlisten()).catch(err => console.error('Failed to unsubscribe from progress event:', err));
      unlistenCompletion?.then(unlisten => unlisten()).catch(err => console.error('Failed to unsubscribe from completion event:', err));
    };
  }, []);

  const handleDownloadStart = async (params: DownloadParams) => {
    setError('');
    setSuccess('');
    setDownloadProgress(0);
    setDownloadInfo({});
    setIsDownloading(true);

    try {
      await invoke('start_download', {
        url: params.url,
        quality: params.quality || undefined,
        format: params.format,
        startTime: params.startTime || undefined,
        endTime: params.endTime || undefined,
        usePlaylist: params.usePlaylist,
        downloadSubtitles: params.downloadSubtitles,
        outputDir: params.outputDir || undefined
      });

      invoke('poll_download_progress');
    } catch (err) {
      console.error('Download failed:', err);
      setIsDownloading(false);
      setError(`Download failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

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
          {error && <Alert type="error" message={error} onDismiss={() => setError('')} />}
          {success && <Alert type="success" message={success} onDismiss={() => setSuccess('')} />}
          
          <Tabs activeTab={activeTab} onChange={setActiveTab} />

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
