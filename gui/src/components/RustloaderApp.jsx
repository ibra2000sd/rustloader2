import React, { useState, useEffect } from 'react';
import { Link, Home, User, AlertCircle, Play, Pause, Download, CheckCircle, XCircle, Clock, RefreshCw, Save, FolderOpen, Settings, Info, HelpCircle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';

// Import our enhanced components
import OnboardingTutorial from './OnboardingTutorial';
import Tooltip from './Tooltip';
import ErrorHandler from './ErrorHandler';
import HelpCenter from './HelpCenter';
import FeatureTour from './FeatureTour';

// Standalone component that doesn't rely on external imports
const RustloaderApp = () => {
  const [activeTab, setActiveTab] = useState('downloads');
  const [isPro, setIsPro] = useState(false);
  const [downloads, setDownloads] = useState([]);
  const [nextId, setNextId] = useState(1);
  const [showProBanner, setShowProBanner] = useState(true);
  const [isAddingUrl, setIsAddingUrl] = useState(false);
  const [newUrl, setNewUrl] = useState('');
  
  // First-run experience state
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [showFeatureTour, setShowFeatureTour] = useState(false);
  const [currentTour, setCurrentTour] = useState('main');
  const [appError, setAppError] = useState('');
  const [isFirstRun, setIsFirstRun] = useState(true);
  const [helpExpanded, setHelpExpanded] = useState(false);
  
  // Settings state for the embedded settings panel
  const [settings, setSettings] = useState({
    downloadPath: '/Users/username/Downloads/rustloader',
    defaultFormat: 'mp4',
    defaultQuality: '720p',
    autoUpdateYtdlp: true,
    enableNotifications: true,
    concurrentDownloads: 2,
    downloadSubtitles: false,
    theme: 'system',
  });

  // Check for first run and show onboarding
  useEffect(() => {
    const checkFirstRun = async () => {
      try {
        // Initialize preferences store
        const store = new Store('preferences.dat');
        await store.load();
        
        // Check if this is the first run
        const hasCompletedOnboarding = await store.get('showOnboarding');
        
        if (hasCompletedOnboarding === undefined || hasCompletedOnboarding === true) {
          // This is the first run or onboarding is enabled
          setShowOnboarding(true);
          setIsFirstRun(true);
        } else {
          setIsFirstRun(false);
        }
      } catch (error) {
        console.error('Failed to check first run state:', error);
        // Default to showing onboarding if we can't determine the state
        setShowOnboarding(true);
      }
    };
    
    checkFirstRun();
  }, []);

  // Start the feature tour after completing onboarding
  const handleOnboardingComplete = () => {
    setShowOnboarding(false);
    // After a short delay, show the feature tour
    setTimeout(() => {
      setCurrentTour('main');
      setShowFeatureTour(true);
    }, 1000);
  };

  // Skip both onboarding and feature tour
  const handleOnboardingSkip = () => {
    setShowOnboarding(false);
  };

  // Handle feature tour completion
  const handleFeatureTourComplete = () => {
    setShowFeatureTour(false);
  };

  // Simulated download progress update
  useEffect(() => {
    if (downloads.length === 0) return;
    
    const interval = setInterval(() => {
      setDownloads(current => 
        current.map(download => {
          if (download.status !== 'downloading' || download.progress >= 100) {
            return download;
          }
          
          // Calculate new progress
          const progressIncrement = Math.random() * 2;
          const newProgress = Math.min(download.progress + progressIncrement, 100);
          
          // Calculate new speed (fluctuating slightly)
          const baseSpeed = download.speed || 1024 * 1024; // 1MB/s
          const speedVariation = baseSpeed * 0.2; // 20% variation
          const newSpeed = baseSpeed + (Math.random() * speedVariation) - (speedVariation/2);
          
          // Calculate remaining size and time
          const remainingSize = download.fileSize * (1 - newProgress/100);
          const timeRemaining = newSpeed > 0 ? remainingSize / newSpeed : 0;
          
          // Update downloaded size
          const downloadedSize = download.fileSize * (newProgress/100);
          
          // Check if download completed
          const newStatus = newProgress >= 100 ? 'complete' : download.status;
          
          return {
            ...download,
            progress: newProgress,
            downloadedSize,
            speed: newSpeed,
            timeRemaining,
            status: newStatus
          };
        })
      );
    }, 1000);
    
    return () => clearInterval(interval);
  }, [downloads]);

  const startDownload = (url) => {
    const videoQualities = ['720p', '480p', '1080p', '4K'];
    const videoFormats = ['mp4', 'webm'];
    
    // Determine if it's a video or audio download
    const isAudio = url.includes('music') || url.includes('audio') || Math.random() > 0.7;
    
    const format = isAudio ? 'mp3' : videoFormats[Math.floor(Math.random() * videoFormats.length)];
    const quality = isAudio 
      ? '128kbps' 
      : videoQualities[Math.floor(Math.random() * (isPro ? 4 : 2))]; // Non-pro users limited to 720p
    
    // Random file size between 10MB and 1GB
    const fileSize = isAudio 
      ? Math.random() * 50 * 1024 * 1024 + 10 * 1024 * 1024 
      : Math.random() * 1024 * 1024 * 1024 + 50 * 1024 * 1024;
    
    // Extract a title from the URL
    let title = '';
    if (url.includes('youtube.com') || url.includes('youtu.be')) {
      title = `YouTube Video ${nextId}`;
    } else if (url.includes('vimeo.com')) {
      title = `Vimeo Video ${nextId}`;
    } else if (url.includes('soundcloud.com')) {
      title = `SoundCloud Track ${nextId}`;
    } else {
      title = `Download ${nextId}`;
    }
    
    const newDownload = {
      id: nextId,
      title,
      url,
      progress: 0,
      fileSize,
      downloadedSize: 0,
      speed: Math.random() * 5 * 1024 * 1024 + 500 * 1024, // 500KB to 5MB/s
      timeRemaining: 0,
      format,
      quality,
      status: 'downloading',
      isPaused: false
    };
    
    setDownloads(current => [...current, newDownload]);
    setNextId(nextId + 1);
    setNewUrl('');
    setIsAddingUrl(false);
  };

  const handleSubmitUrl = (e) => {
    e.preventDefault();
    if (!newUrl.trim()) {
      setAppError('Please enter a URL to download');
      return;
    }
    
    // Validate URL
    try {
      const url = new URL(newUrl);
      
      // Check for supported domains
      const supportedDomains = ['youtube.com', 'youtu.be', 'vimeo.com', 'dailymotion.com', 'soundcloud.com'];
      const isSupported = supportedDomains.some(domain => url.hostname.includes(domain));
      
      if (!isSupported) {
        setAppError(`This URL may not be supported. We support YouTube, Vimeo, and other popular platforms. Continue anyway?`);
        // For demo purposes, we'll still allow the download
        startDownload(newUrl);
      } else {
        startDownload(newUrl);
      }
    } catch (error) {
      setAppError('Invalid URL. Please enter a valid web address.');
    }
  };

  const handlePauseResume = (id) => {
    setDownloads(current => 
      current.map(download => {
        if (download.id === id) {
          const isPaused = !download.isPaused;
          return {
            ...download,
            isPaused,
            status: isPaused ? 'paused' : 'downloading'
          };
        }
        return download;
      })
    );
  };

  const handleCancel = (id) => {
    setDownloads(current => current.filter(download => download.id !== id));
  };

  const handleSaveSettings = () => {
    console.log('Settings saved:', settings);
    // In a real app, this would save settings to storage
    alert('Settings saved successfully!');
  };

  const handleCheckForUpdates = () => {
    // In a real app, this would check for updates
    alert('Checking for updates... No updates available.');
  };

  const handleUpgradeToPro = () => {
    // In a real app, this would open a payment flow
    alert('This would open the upgrade flow in the actual application.');
    // For demo purposes, let's toggle Pro mode
    setIsPro(!isPro);
  };

  const handleSettingChange = (e) => {
    const { name, value, type, checked } = e.target;
    setSettings({
      ...settings,
      [name]: type === 'checkbox' ? checked : value
    });
  };

  const handleSelectDirectory = () => {
    // In a real app, this would use a file dialog API
    alert('In the actual app, this would open a file dialog to select download directory');
  };

  // Utility function to format file sizes
  const formatBytes = (bytes, decimals = 2) => {
    if (bytes === 0) return '0 Bytes';
    
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
  };

  // Utility function to format time
  const formatTime = (seconds) => {
    if (!seconds) return '--:--';
    
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    
    return `${String(minutes).padStart(2, '0')}:${String(remainingSeconds).padStart(2, '0')}`;
  };

  // Sample URLs for the demo
  const sampleUrls = [
    'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
    'https://www.youtube.com/watch?v=9bZkp7q19f0',
    'https://vimeo.com/148751763',
    'https://soundcloud.com/artist/track'
  ];

  // Embedded DownloadInfoCard component
  const DownloadInfoCard = ({ downloadInfo, isPaused, onPauseResume, onCancel }) => {
    const {
      title = 'Downloading...',
      progress = 0,
      fileSize = 0,
      downloadedSize = 0,
      speed = 0,
      timeRemaining = 0,
      format = 'mp4',
      quality = '720p',
      status = 'downloading'
    } = downloadInfo;

    return (
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden max-w-2xl w-full">
        {/* Header Section with Title and Status */}
        <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
          <div className="flex justify-between items-center">
            <h3 className="font-medium text-gray-800 dark:text-gray-200 truncate">
              {title}
            </h3>
            <span className={`px-2 py-1 text-xs font-medium rounded-full ${
              status === 'complete' ? 'bg-green-100 text-green-800 dark:bg-green-800 dark:text-green-100' :
              status === 'error' ? 'bg-red-100 text-red-800 dark:bg-red-800 dark:text-red-100' :
              status === 'paused' ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-800 dark:text-yellow-100' :
              'bg-blue-100 text-blue-800 dark:bg-blue-800 dark:text-blue-100'
            }`}>
              {status === 'complete' ? 'Complete' :
               status === 'error' ? 'Error' :
               status === 'paused' ? 'Paused' : 'Downloading'}
            </span>
          </div>
        </div>
        
        {/* Progress Bar */}
        <div className="px-4 pt-4">
          <div className="relative w-full h-4 bg-gray-200 dark:bg-gray-600 rounded-full overflow-hidden">
            <div
              className={`absolute left-0 top-0 h-full rounded-full transition-all duration-300 ${
                status === 'complete' ? 'bg-green-500 dark:bg-green-600' :
                status === 'error' ? 'bg-red-500 dark:bg-red-600' :
                status === 'paused' ? 'bg-yellow-500 dark:bg-yellow-600' :
                'bg-blue-500 dark:bg-blue-600'
              }`}
              style={{ width: `${progress}%` }}
            ></div>
          </div>
          <div className="flex justify-between mt-1 text-xs text-gray-500 dark:text-gray-400">
            <span>{formatBytes(downloadedSize)}</span>
            <span>{progress.toFixed(1)}%</span>
            <span>{formatBytes(fileSize)}</span>
          </div>
        </div>
        
        {/* Stats Grid */}
        <div className="grid grid-cols-2 gap-4 p-4">
          <div className="flex items-center space-x-2">
            <Download size={16} className="text-gray-500 dark:text-gray-400" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Speed</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {formatBytes(speed)}/s
              </p>
            </div>
          </div>
          
          <div className="flex items-center space-x-2">
            <Clock size={16} className="text-gray-500 dark:text-gray-400" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Time Remaining</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {formatTime(timeRemaining)}
              </p>
            </div>
          </div>
          
          <div className="flex items-center space-x-2">
            <div className="w-4 h-4 rounded-full bg-blue-500 flex items-center justify-center">
              <span className="text-white text-xs font-bold">{format.toUpperCase().charAt(0)}</span>
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Format</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {format.toUpperCase()}
              </p>
            </div>
          </div>
          
          <div className="flex items-center space-x-2">
            <div className="w-4 h-4 rounded-full bg-purple-500 flex items-center justify-center">
              <span className="text-white text-xs font-bold">Q</span>
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Quality</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {quality}
              </p>
            </div>
          </div>
        </div>
        
        {/* Action Buttons */}
        <div className="flex justify-between p-4 bg-gray-50 dark:bg-gray-700 border-t border-gray-200 dark:border-gray-600">
          <button
            onClick={onPauseResume}
            disabled={status === 'complete' || status === 'error'}
            className={`px-4 py-2 rounded-md text-sm font-medium
              ${(status === 'complete' || status === 'error') 
                ? 'bg-gray-200 text-gray-500 dark:bg-gray-600 dark:text-gray-400 cursor-not-allowed' 
                : 'bg-blue-100 text-blue-700 hover:bg-blue-200 dark:bg-blue-700 dark:text-blue-100 dark:hover:bg-blue-600'}
            `}
          >
            {isPaused ? (
              <div className="flex items-center space-x-1">
                <Play size={16} />
                <span>Resume</span>
              </div>
            ) : (
              <div className="flex items-center space-x-1">
                <Pause size={16} />
                <span>Pause</span>
              </div>
            )}
          </button>
          
          <button
            onClick={onCancel}
            disabled={status === 'complete'}
            className={`px-4 py-2 rounded-md text-sm font-medium
              ${status === 'complete'
                ? 'bg-gray-200 text-gray-500 dark:bg-gray-600 dark:text-gray-400 cursor-not-allowed'
                : 'bg-red-100 text-red-700 hover:bg-red-200 dark:bg-red-700 dark:text-red-100 dark:hover:bg-red-600'}
            `}
          >
            <div className="flex items-center space-x-1">
              <XCircle size={16} />
              <span>Cancel</span>
            </div>
          </button>
          
          {status === 'complete' && (
            <button
              className="px-4 py-2 rounded-md text-sm font-medium bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-700 dark:text-green-100 dark:hover:bg-green-600"
            >
              <div className="flex items-center space-x-1">
                <CheckCircle size={16} />
                <span>Open File</span>
              </div>
            </button>
          )}
        </div>
      </div>
    );
  };

  // Embedded simplified Settings Panel
  const SettingsPanel = () => {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden max-w-2xl w-full">
        {/* Header */}
        <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
          <div className="flex items-center space-x-2">
            <Settings className="text-gray-500 dark:text-gray-400" size={20} />
            <h2 className="font-medium text-gray-800 dark:text-gray-200">Settings</h2>
          </div>
        </div>
        
        {/* Settings Content */}
        <div className="p-4">
          {/* Download Location */}
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Download Location
            </label>
            <div className="flex">
              <input
                type="text"
                name="downloadPath"
                value={settings.downloadPath}
                onChange={handleSettingChange}
                className="flex-grow px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-l-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
                placeholder="Download path"
              />
              <button
                onClick={handleSelectDirectory}
                type="button"
                className="px-3 py-2 bg-gray-100 dark:bg-gray-600 text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-gray-600 rounded-r-md hover:bg-gray-200 dark:hover:bg-gray-500"
              >
                <FolderOpen size={18} />
              </button>
            </div>
          </div>
          
          {/* Grid layout for settings */}
          <div className="grid grid-cols-2 gap-4 mb-4">
            {/* Default Format */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Default Format
              </label>
              <select
                name="defaultFormat"
                value={settings.defaultFormat}
                onChange={handleSettingChange}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
              >
                <option value="mp4">MP4</option>
                <option value="webm">WebM</option>
                <option value="mp3">MP3</option>
                {isPro && <option value="mkv">MKV</option>}
                {isPro && <option value="flac">FLAC</option>}
              </select>
            </div>
            
            {/* Default Quality */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Default Quality
              </label>
              <select
                name="defaultQuality"
                value={settings.defaultQuality}
                onChange={handleSettingChange}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
              >
                <option value="480p">480p</option>
                <option value="720p">720p</option>
                {isPro && <option value="1080p">1080p</option>}
                {isPro && <option value="4K">4K</option>}
              </select>
            </div>
          </div>
          
          {/* Checkboxes */}
          <div className="space-y-3 mb-4">
            <div className="flex items-center">
              <input
                type="checkbox"
                id="autoUpdateYtdlp"
                name="autoUpdateYtdlp"
                checked={settings.autoUpdateYtdlp}
                onChange={handleSettingChange}
                className="h-4 w-4 text-blue-600 dark:text-blue-500 focus:ring-blue-500 border-gray-300 dark:border-gray-600 rounded"
              />
              <label htmlFor="autoUpdateYtdlp" className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
                Auto-update yt-dlp on startup
              </label>
            </div>
            
            <div className="flex items-center">
              <input
                type="checkbox"
                id="enableNotifications"
                name="enableNotifications"
                checked={settings.enableNotifications}
                onChange={handleSettingChange}
                className="h-4 w-4 text-blue-600 dark:text-blue-500 focus:ring-blue-500 border-gray-300 dark:border-gray-600 rounded"
              />
              <label htmlFor="enableNotifications" className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
                Show desktop notifications
              </label>
            </div>
            
            <div className="flex items-center">
              <input
                type="checkbox"
                id="downloadSubtitles"
                name="downloadSubtitles"
                checked={settings.downloadSubtitles}
                onChange={handleSettingChange}
                className="h-4 w-4 text-blue-600 dark:text-blue-500 focus:ring-blue-500 border-gray-300 dark:border-gray-600 rounded"
              />
              <label htmlFor="downloadSubtitles" className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
                Download subtitles when available
              </label>
            </div>
          </div>
        </div>
        
        {/* Footer */}
        <div className="flex justify-between p-4 bg-gray-50 dark:bg-gray-700 border-t border-gray-200 dark:border-gray-600">
          <button
            onClick={handleCheckForUpdates}
            type="button"
            className="flex items-center space-x-1 px-4 py-2 bg-gray-100 dark:bg-gray-600 text-gray-700 dark:text-gray-200 rounded-md hover:bg-gray-200 dark:hover:bg-gray-500"
          >
            <RefreshCw size={16} />
            <span>Check for Updates</span>
          </button>
          
          <button
            onClick={handleSaveSettings}
            type="button"
            className="flex items-center space-x-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
          >
            <Save size={16} />
            <span>Save Settings</span>
          </button>
        </div>
      </div>
    );
  };

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      {/* App Header */}
      <header className="bg-white dark:bg-gray-800 shadow-sm">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between h-16">
            <div className="flex items-center">
              <div className="flex-shrink-0 flex items-center">
                {/* App Logo */}
                <div className="h-8 w-8 rounded-md bg-blue-600 flex items-center justify-center mr-2">
                  <Download className="h-5 w-5 text-white" />
                </div>
                <span className="text-xl font-bold text-gray-900 dark:text-white">Rustloader</span>
                {isPro && (
                  <span className="ml-2 px-2 py-1 text-xs font-medium rounded-full bg-yellow-100 text-yellow-800 dark:bg-yellow-800 dark:text-yellow-100">
                    PRO
                  </span>
                )}
              </div>
            </div>
            
            <div className="flex items-center">
              {!isPro && (
                <button
                  onClick={handleUpgradeToPro}
                  className="mr-4 px-3 py-1 text-sm font-medium rounded-md bg-gradient-to-r from-yellow-400 to-yellow-600 text-white hover:from-yellow-500 hover:to-yellow-700 transition-all"
                >
                  Upgrade to Pro
                </button>
              )}
              
              <Tooltip 
                content="Access your account settings and license information"
                position="bottom"
              >
                <button className="p-2 rounded-md text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300">
                  <User size={20} />
                </button>
              </Tooltip>
              
              <Tooltip
                content="Open the quick help guide"
                position="bottom"
              >
                <button
                  className="p-2 ml-2 rounded-md text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
                  onClick={() => setShowOnboarding(true)}
                >
                  <HelpCircle size={20} />
                </button>
              </Tooltip>
            </div>
          </div>
        </div>
      </header>
      
      {/* Main Content */}
      <main className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
        {/* Error Message */}
        {appError && (
          <div className="mb-6">
            <ErrorHandler
              error={appError}
              onDismiss={() => setAppError('')}
              actions={[
                {
                  label: 'Dismiss',
                  action: () => setAppError(''),
                  primary: true
                }
              ]}
            />
          </div>
        )}
        
        {/* Pro Version Banner */}
        {!isPro && showProBanner && (
          <div className="mb-6 p-4 bg-gradient-to-r from-blue-500 to-indigo-600 rounded-lg shadow-md text-white relative">
            <button 
              onClick={() => setShowProBanner(false)}
              className="absolute top-2 right-2 text-white hover:text-gray-200"
            >
              ×
            </button>
            <div className="flex flex-col sm:flex-row items-center">
              <div className="mb-4 sm:mb-0 sm:mr-6 flex-shrink-0">
                <span className="inline-flex items-center justify-center h-12 w-12 rounded-md bg-white bg-opacity-10">
                  <AlertCircle className="h-6 w-6 text-white" />
                </span>
              </div>
              <div className="text-center sm:text-left">
                <h3 className="text-lg font-medium">Unlock Rustloader Pro</h3>
                <p className="mt-1 text-sm text-blue-100">
                  Get 4K downloads, unlimited concurrent downloads, high-quality audio formats and more!
                </p>
              </div>
              <div className="mt-4 sm:mt-0 sm:ml-auto">
                <button
                  onClick={handleUpgradeToPro}
                  className="px-4 py-2 bg-white text-blue-600 rounded-md font-medium hover:bg-blue-50 focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-blue-600"
                >
                  Upgrade Now
                </button>
              </div>
            </div>
          </div>
        )}
        
        {/* Tab Navigation */}
        <div className="mb-6">
          <div className="border-b border-gray-200 dark:border-gray-700">
            <nav className="-mb-px flex space-x-8">
              <button
                onClick={() => setActiveTab('downloads')}
                className={`py-2 px-1 border-b-2 font-medium text-sm ${
                  activeTab === 'downloads' 
                    ? 'border-blue-500 text-blue-600 dark:text-blue-400' 
                    : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                }`}
                id="downloads-tab"
              >
                <div className="flex items-center">
                  <Home size={16} className="mr-2" />
                  Downloads
                </div>
              </button>
              
              <button
                onClick={() => setActiveTab('settings')}
                className={`py-2 px-1 border-b-2 font-medium text-sm ${
                  activeTab === 'settings' 
                    ? 'border-blue-500 text-blue-600 dark:text-blue-400' 
                    : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                }`}
                id="settings-tab"
              >
                <div className="flex items-center">
                  <Settings size={16} className="mr-2" />
                  Settings
                </div>
              </button>
              
              <button
                onClick={() => {
                  setActiveTab('help');
                  setHelpExpanded(true);
                }}
                className={`py-2 px-1 border-b-2 font-medium text-sm ${
                  activeTab === 'help' 
                    ? 'border-blue-500 text-blue-600 dark:text-blue-400' 
                    : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                }`}
                id="help-tab"
              >
                <div className="flex items-center">
                  <Link size={16} className="mr-2" />
                  Help
                </div>
              </button>
            </nav>
          </div>
        </div>
        
        {/* Tab Content */}
        <div>
          {/* Downloads Tab */}
          {activeTab === 'downloads' && (
            <div>
              {/* URL Input */}
              <div className="mb-6">
                {isAddingUrl ? (
                  <form onSubmit={handleSubmitUrl} className="flex">
                    <input
                      type="text"
                      value={newUrl}
                      onChange={(e) => setNewUrl(e.target.value)}
                      placeholder="Paste URL here (YouTube, Vimeo, SoundCloud, etc.)"
                      className="flex-grow px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-l-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
                      autoFocus
                      id="url-input-field"
                    />
                    <Tooltip
                      content="Start downloading the video or audio from this URL"
                      position="bottom"
                    >
                      <button
                        type="submit"
                        className="px-4 py-2 bg-blue-600 text-white rounded-r-md hover:bg-blue-700"
                        id="download-button"
                      >
                        Download
                      </button>
                    </Tooltip>
                  </form>
                ) : (
                  <button
                    onClick={() => setIsAddingUrl(true)}
                    className="w-full flex justify-center items-center px-4 py-3 border border-gray-300 dark:border-gray-600 border-dashed rounded-md text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
                  >
                    <Download size={18} className="mr-2" />
                    <span>Add URL to download</span>
                  </button>
                )}
              </div>
              
              {/* Format and Quality Selection (visible when isFirstRun and no downloads to guide user) */}
              {isFirstRun && downloads.length === 0 && (
                <div className="mb-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                  <h3 className="text-blue-800 dark:text-blue-300 font-medium mb-2">Quick Settings</h3>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div>
                      <label className="block text-sm text-blue-700 dark:text-blue-300 mb-1">
                        Format
                        <Tooltip 
                          content="Select MP4/WebM for video or MP3/M4A for audio only"
                          position="right"
                          icon={true}
                          className="ml-1"
                        />
                      </label>
                      <select 
                        className="w-full px-3 py-2 border border-blue-300 dark:border-blue-700 rounded-md bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100"
                        id="format-selector"
                      >
                        <option value="mp4">MP4 Video</option>
                        <option value="webm">WebM Video</option>
                        <option value="mp3">MP3 Audio</option>
                        <option value="m4a">M4A Audio</option>
                      </select>
                    </div>
                    <div>
                      <label className="block text-sm text-blue-700 dark:text-blue-300 mb-1">
                        Quality
                        <Tooltip 
                          content="Higher quality means larger file size and longer download time"
                          position="right"
                          icon={true}
                          className="ml-1"
                        />
                      </label>
                      <select 
                        className="w-full px-3 py-2 border border-blue-300 dark:border-blue-700 rounded-md bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100"
                        id="quality-selector"
                      >
                        <option value="best">Best</option>
                        <option value="720p">720p</option>
                        <option value="480p">480p</option>
                        <option value="360p">360p</option>
                      </select>
                    </div>
                  </div>
                </div>
              )}
              
              {/* Downloads List */}
              <div className="space-y-6" id="downloads-list">
                {downloads.length === 0 ? (
                  <div>
                    <div className="text-center p-8 bg-gray-50 dark:bg-gray-800 rounded-lg border border-dashed border-gray-300 dark:border-gray-700">
                      <p className="text-gray-500 dark:text-gray-400 mb-4">No active downloads</p>
                      
                      <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Try downloading from:</h3>
                      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-2">
                        {sampleUrls.map((url, index) => (
                          <button
                            key={index}
                            onClick={() => startDownload(url)}
                            className="text-xs text-left py-2 px-3 bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded hover:bg-gray-50 dark:hover:bg-gray-600 truncate"
                          >
                            {url}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                ) : (
                  downloads.map(download => (
                    <DownloadInfoCard
                      key={download.id}
                      downloadInfo={download}
                      isPaused={download.isPaused}
                      onPauseResume={() => handlePauseResume(download.id)}
                      onCancel={() => handleCancel(download.id)}
                    />
                  ))
                )}
              </div>
              
              {/* Free Version Limit Info */}
              {!isPro && downloads.length > 0 && (
                <div className="mt-6 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 text-gray-500 dark:text-gray-400 text-sm flex justify-between items-center">
                  <div>
                    <span>Free version limit: </span>
                    <span className="font-medium">{downloads.length}/5 downloads today</span>
                  </div>
                  <button
                    onClick={handleUpgradeToPro}
                    className="px-3 py-1 text-xs font-medium rounded-md bg-blue-600 text-white hover:bg-blue-700"
                  >
                    Upgrade for unlimited
                  </button>
                </div>
              )}
            </div>
          )}
          
          {/* Settings Tab */}
          {activeTab === 'settings' && (
            <SettingsPanel />
          )}
          
          {/* Help Tab */}
          {activeTab === 'help' && (
            <HelpCenter />
          )}
        </div>
      </main>
      
      {/* Footer */}
      <footer className="bg-white dark:bg-gray-800 shadow-inner mt-6">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex flex-col sm:flex-row justify-between items-center">
            <div className="text-sm text-gray-500 dark:text-gray-400 mb-2 sm:mb-0">
              Rustloader v1.0.0 &copy; 2025
            </div>
            <div className="flex space-x-4">
              <a href="#" className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300">
                Privacy Policy
              </a>
              <a href="#" className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300">
                Terms of Service
              </a>
              <a href="#" className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300">
                License
              </a>
            </div>
          </div>
        </div>
      </footer>
      
      {/* Onboarding Tutorial Modal */}
      {showOnboarding && (
        <OnboardingTutorial
          onComplete={handleOnboardingComplete}
          onSkip={handleOnboardingSkip}
        />
      )}
      
      {/* Feature Tour */}
      {showFeatureTour && (
        <FeatureTour
          onComplete={handleFeatureTourComplete}
          onSkip={handleFeatureTourComplete}
          featureKey={currentTour}
        />
      )}
    </div>
  );
};

export default RustloaderApp;