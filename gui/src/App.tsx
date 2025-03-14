// src/App.tsx

import React, { useState, useEffect } from 'react';
import './App.css';
import DownloadForm from './components/DownloadForm';
import ProgressBar from './components/ProgressBar';
import LicenseInfo from './components/LicenseInfo';

// Import Tauri API functions
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

// Define app state type
interface AppState {
  licenseStatus: 'checking' | 'free' | 'pro';
  isDownloading: boolean;
  downloadProgress: number;
  activeTab: 'download' | 'license';
  errorMessage: string;
  successMessage: string;
}

// Main App component with improved error handling
const App: React.FC = () => {
  // Initialize state with proper typing
  const [state, setState] = useState<AppState>({
    licenseStatus: 'checking',
    isDownloading: false,
    downloadProgress: 0,
    activeTab: 'download',
    errorMessage: '',
    successMessage: ''
  });

  // Check license status on mount
  useEffect(() => {
    const checkLicense = async () => {
      try {
        const status = await invoke<string>('check_license');
        setState(prev => ({ ...prev, licenseStatus: status as 'free' | 'pro' }));
      } catch (error) {
        console.error('Failed to check license:', error);
        setState(prev => ({ 
          ...prev, 
          licenseStatus: 'free',
          errorMessage: `License check failed: ${error}. Using free version.` 
        }));
      }
    };

    checkLicense();
  }, []);

  // Listen for download progress events from Rust backend
  useEffect(() => {
    const unsubscribe = listen<number>('download-progress', (event) => {
      setState(prev => ({ ...prev, downloadProgress: event.payload }));

      // When download completes
      if (event.payload >= 100) {
        setTimeout(() => {
          setState(prev => ({ 
            ...prev, 
            isDownloading: false,
            successMessage: 'Download completed successfully!'
          }));
        }, 2000); // Give time for processing to complete
      }
    });

    // Cleanup listener on unmount
    return () => {
      unsubscribe.then(unsub => unsub());
    };
  }, []);

  // Check for pending downloads on mount (useful for page refreshes)
  useEffect(() => {
    const checkPendingDownloads = async () => {
      try {
        const isDownloading = await invoke<boolean>('check_pending_downloads');
        if (isDownloading) {
          setState(prev => ({ 
            ...prev, 
            isDownloading: true,
            downloadProgress: 0
          }));
        }
      } catch (error) {
        console.error('Failed to check pending downloads:', error);
      }
    };

    checkPendingDownloads();
  }, []);

  // Handle download form submission
  const handleDownloadStart = async (downloadParams: {
    url: string;
    quality: string;
    format: string;
    startTime?: string;
    endTime?: string;
    usePlaylist: boolean;
    downloadSubtitles: boolean;
    outputDir?: string;
  }) => {
    // Clear previous messages
    setState(prev => ({ 
      ...prev, 
      errorMessage: '',
      successMessage: ''
    }));

    try {
      setState(prev => ({ ...prev, isDownloading: true, downloadProgress: 0 }));
      
      await invoke('download_video', {
        url: downloadParams.url,
        quality: downloadParams.quality || undefined,
        format: downloadParams.format,
        startTime: downloadParams.startTime || undefined,
        endTime: downloadParams.endTime || undefined,
        usePlaylist: downloadParams.usePlaylist,
        downloadSubtitles: downloadParams.downloadSubtitles,
        outputDir: downloadParams.outputDir || undefined
      });
      
      // Note: We don't set isDownloading to false here because we'll
      // receive progress events that will update the state
    } catch (error) {
      console.error('Download failed:', error);
      setState(prev => ({ 
        ...prev, 
        isDownloading: false,
        errorMessage: `Download failed: ${error}`
      }));
    }
  };

  // Handle license activation
  const handleLicenseActivation = async (licenseKey: string, email: string) => {
    // Clear previous messages
    setState(prev => ({ 
      ...prev, 
      errorMessage: '',
      successMessage: ''
    }));

    try {
      const result = await invoke<string>('activate_license_key', {
        licenseKey,
        email
      });
      
      setState(prev => ({ 
        ...prev, 
        licenseStatus: 'pro',
        successMessage: result || 'License activated successfully!'
      }));
    } catch (error) {
      console.error('License activation failed:', error);
      setState(prev => ({ 
        ...prev, 
        errorMessage: `License activation failed: ${error}`
      }));
    }
  };

  // Dismiss messages
  const dismissMessage = (type: 'error' | 'success') => {
    if (type === 'error') {
      setState(prev => ({ ...prev, errorMessage: '' }));
    } else {
      setState(prev => ({ ...prev, successMessage: '' }));
    }
  };

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-gray-900 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <header className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-800 dark:text-white mb-2">
            Rustloader
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Advanced Video Downloader
          </p>
          
          {/* License Badge */}
          <div className="mt-3">
            <span className={`inline-block px-3 py-1 text-sm font-medium text-white rounded-full ${
              state.licenseStatus === 'checking' ? 'bg-gray-500' :
              state.licenseStatus === 'pro' ? 'bg-yellow-500' : 'bg-blue-500'
            }`}>
              {state.licenseStatus === 'checking' ? 'CHECKING LICENSE...' :
               state.licenseStatus === 'pro' ? 'PRO VERSION' : 'FREE VERSION'}
            </span>
          </div>
        </header>
        
        {/* Error and Success Messages */}
        {state.errorMessage && (
          <div className="mb-6 bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative" role="alert">
            <strong className="font-bold">Error! </strong>
            <span className="block sm:inline">{state.errorMessage}</span>
            <span 
              className="absolute top-0 bottom-0 right-0 px-4 py-3 cursor-pointer"
              onClick={() => dismissMessage('error')}
            >
              <svg className="fill-current h-6 w-6 text-red-500" role="button" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                <title>Close</title>
                <path d="M14.348 14.849a1.2 1.2 0 0 1-1.697 0L10 11.819l-2.651 3.029a1.2 1.2 0 1 1-1.697-1.697l2.758-3.15-2.759-3.152a1.2 1.2 0 1 1 1.697-1.697L10 8.183l2.651-3.031a1.2 1.2 0 1 1 1.697 1.697l-2.758 3.152 2.758 3.15a1.2 1.2 0 0 1 0 1.698z"/>
              </svg>
            </span>
          </div>
        )}
        
        {state.successMessage && (
          <div className="mb-6 bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded relative" role="alert">
            <strong className="font-bold">Success! </strong>
            <span className="block sm:inline">{state.successMessage}</span>
            <span 
              className="absolute top-0 bottom-0 right-0 px-4 py-3 cursor-pointer"
              onClick={() => dismissMessage('success')}
            >
              <svg className="fill-current h-6 w-6 text-green-500" role="button" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                <title>Close</title>
                <path d="M14.348 14.849a1.2 1.2 0 0 1-1.697 0L10 11.819l-2.651 3.029a1.2 1.2 0 1 1-1.697-1.697l2.758-3.15-2.759-3.152a1.2 1.2 0 1 1 1.697-1.697L10 8.183l2.651-3.031a1.2 1.2 0 1 1 1.697 1.697l-2.758 3.152 2.758 3.15a1.2 1.2 0 0 1 0 1.698z"/>
              </svg>
            </span>
          </div>
        )}
        
        {/* Tab Navigation */}
        <div className="flex border-b border-gray-200 dark:border-gray-700 mb-6">
          <button
            className={`py-2 px-4 font-medium text-sm ${
              state.activeTab === 'download'
                ? 'text-blue-600 dark:text-blue-400 border-b-2 border-blue-600 dark:border-blue-400'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
            }`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'download' }))}
          >
            Download
          </button>
          <button
            className={`py-2 px-4 font-medium text-sm ${
              state.activeTab === 'license'
                ? 'text-blue-600 dark:text-blue-400 border-b-2 border-blue-600 dark:border-blue-400'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
            }`}
            onClick={() => setState(prev => ({ ...prev, activeTab: 'license' }))}
          >
            License
          </button>
        </div>
        
        {/* Main Content */}
        <div className="space-y-6">
          {/* Progress Bar (shown during download) */}
          {state.isDownloading && (
            <ProgressBar progress={state.downloadProgress} />
          )}
          
          {/* Active Tab Content */}
          {state.activeTab === 'download' ? (
            <DownloadForm 
              isPro={state.licenseStatus === 'pro'} 
              onDownloadStart={handleDownloadStart}
              disabled={state.isDownloading}
            />
          ) : (
            <LicenseInfo 
              isProVersion={state.licenseStatus === 'pro'} 
              onActivationComplete={(success) => {
                if (success) {
                  setState(prev => ({ ...prev, licenseStatus: 'pro' }));
                }
              }}
            />
          )}
          
          {/* Info Card */}
          <div className="bg-blue-50 dark:bg-blue-900 p-4 rounded-lg shadow-sm">
            <h3 className="text-sm font-medium text-blue-800 dark:text-blue-200 mb-1">
              Rustloader v1.0.0
            </h3>
            <p className="text-xs text-blue-600 dark:text-blue-300">
              Advanced Video Downloader built with Rust and React
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default App;