import React, { useState, useEffect } from 'react';
import { Settings, Save, RefreshCw, FolderOpen, AlertCircle, Bell, BellOff } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

const SettingsPanel = ({ 
  isPro = false,
  onSave = () => {},
  onCheckForUpdates = () => {}
}) => {
  const [settings, setSettings] = useState({
    downloadPath: '/Users/username/Downloads/rustloader',
    defaultFormat: 'mp4',
    defaultQuality: '720p',
    autoUpdateYtdlp: true,
    enableNotifications: true,
    concurrentDownloads: 2,
    downloadSubtitles: false,
    createPlaylist: false,
    theme: 'system',
    skipExisting: true,
    customUserAgent: '',
  });
  
  const [isExpanded, setIsExpanded] = useState(true);
  const [notificationsSupported, setNotificationsSupported] = useState(false);
  const [permissionRequested, setPermissionRequested] = useState(false);
  
  // Check if notifications are supported
  useEffect(() => {
    const checkNotificationSupport = async () => {
      try {
        const supported = await invoke('are_notifications_supported');
        setNotificationsSupported(supported);
        
        if (!supported) {
          // If not supported, disable the notification option
          setSettings(prev => ({
            ...prev,
            enableNotifications: false
          }));
        }
      } catch (error) {
        console.error('Error checking notification support:', error);
        setNotificationsSupported(false);
      }
    };
    
    checkNotificationSupport();
  }, []);
  
  const handleChange = async (e) => {
    const { name, value, type, checked } = e.target;
    
    // Special handling for notifications
    if (name === 'enableNotifications') {
      // If enabling notifications and not previously requested permission
      if (checked && !permissionRequested && notificationsSupported) {
        try {
          // Request permission
          const granted = await invoke('request_notification_permission');
          setPermissionRequested(true);
          
          if (!granted) {
            // Permission denied
            console.log('Notification permission denied');
            // Update the settings without changing notification preference
            setSettings(prev => ({
              ...prev,
              enableNotifications: false
            }));
            // Notify the backend
            await invoke('toggle_notifications', { enabled: false });
            return;
          }
        } catch (error) {
          console.error('Error requesting notification permission:', error);
          // Keep notifications disabled on error
          setSettings(prev => ({
            ...prev,
            enableNotifications: false
          }));
          return;
        }
      }
      
      // Update notification setting in the backend
      try {
        await invoke('toggle_notifications', { enabled: checked });
      } catch (error) {
        console.error('Error toggling notifications:', error);
      }
    }
    
    // Update local state
    setSettings({
      ...settings,
      [name]: type === 'checkbox' ? checked : value
    });
  };
  
  const handleSave = () => {
    onSave(settings);
  };
  
  const handleSelectDirectory = async () => {
    // In a real app, this would use a file dialog API
    alert('In the actual app, this would open a file dialog to select download directory');
  };
  
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden max-w-2xl w-full">
      {/* Header */}
      <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600 flex justify-between items-center cursor-pointer" 
        onClick={() => setIsExpanded(!isExpanded)}>
        <div className="flex items-center space-x-2">
          <Settings className="text-gray-500 dark:text-gray-400" size={20} />
          <h2 className="font-medium text-gray-800 dark:text-gray-200">Settings</h2>
        </div>
        <div>
          <button className="text-gray-500 dark:text-gray-400">
            {isExpanded ? '▼' : '▶'}
          </button>
        </div>
      </div>
      
      {/* Settings Content */}
      {isExpanded && (
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
                onChange={handleChange}
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
                onChange={handleChange}
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
                onChange={handleChange}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
              >
                <option value="480p">480p</option>
                <option value="720p">720p</option>
                {isPro && <option value="1080p">1080p</option>}
                {isPro && <option value="4K">4K</option>}
              </select>
            </div>
            
            {/* Concurrent Downloads */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Concurrent Downloads
              </label>
              <select
                name="concurrentDownloads"
                value={settings.concurrentDownloads}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
              >
                <option value={1}>1</option>
                <option value={2}>2</option>
                {isPro && <option value={3}>3</option>}
                {isPro && <option value={4}>4</option>}
              </select>
            </div>
            
            {/* Theme */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Theme
              </label>
              <select
                name="theme"
                value={settings.theme}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
              >
                <option value="light">Light</option>
                <option value="dark">Dark</option>
                <option value="system">System</option>
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
                onChange={handleChange}
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
                onChange={handleChange}
                disabled={!notificationsSupported}
                className={`h-4 w-4 text-blue-600 dark:text-blue-500 focus:ring-blue-500 border-gray-300 dark:border-gray-600 rounded
                  ${!notificationsSupported ? 'opacity-50 cursor-not-allowed' : ''}`}
              />
              <label htmlFor="enableNotifications" className="ml-2 flex items-center gap-1 text-sm text-gray-700 dark:text-gray-300">
                {settings.enableNotifications ? <Bell size={16} /> : <BellOff size={16} />}
                Show desktop notifications
                {!notificationsSupported && (
                  <span className="ml-2 text-xs text-red-500 dark:text-red-400 italic">
                    (Not supported in this browser/OS)
                  </span>
                )}
              </label>
            </div>
            
            <div className="flex items-center">
              <input
                type="checkbox"
                id="downloadSubtitles"
                name="downloadSubtitles"
                checked={settings.downloadSubtitles}
                onChange={handleChange}
                className="h-4 w-4 text-blue-600 dark:text-blue-500 focus:ring-blue-500 border-gray-300 dark:border-gray-600 rounded"
              />
              <label htmlFor="downloadSubtitles" className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
                Download subtitles when available
              </label>
            </div>
            
            <div className="flex items-center">
              <input
                type="checkbox"
                id="skipExisting"
                name="skipExisting"
                checked={settings.skipExisting}
                onChange={handleChange}
                className="h-4 w-4 text-blue-600 dark:text-blue-500 focus:ring-blue-500 border-gray-300 dark:border-gray-600 rounded"
              />
              <label htmlFor="skipExisting" className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
                Skip existing files
              </label>
            </div>
            
            <div className="flex items-center">
              <input
                type="checkbox"
                id="createPlaylist"
                name="createPlaylist"
                checked={settings.createPlaylist}
                onChange={handleChange}
                className="h-4 w-4 text-blue-600 dark:text-blue-500 focus:ring-blue-500 border-gray-300 dark:border-gray-600 rounded"
              />
              <label htmlFor="createPlaylist" className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
                Create playlist when downloading multiple videos
              </label>
            </div>
          </div>
          
          {/* Advanced Section */}
          <div className="mb-4">
            <details>
              <summary className="text-sm font-medium text-gray-700 dark:text-gray-300 cursor-pointer">
                Advanced Settings
              </summary>
              <div className="mt-2 pl-4 border-l-2 border-gray-200 dark:border-gray-700">
                <div className="mb-3">
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    Custom User Agent
                  </label>
                  <input
                    type="text"
                    name="customUserAgent"
                    value={settings.customUserAgent}
                    onChange={handleChange}
                    placeholder="Leave empty for default"
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
                  />
                </div>
                
                {!isPro && (
                  <div className="p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded text-sm text-yellow-700 dark:text-yellow-300 flex items-start space-x-2">
                    <AlertCircle className="flex-shrink-0 h-5 w-5 text-yellow-400 dark:text-yellow-500 mt-0.5" />
                    <div>
                      <p><strong>Pro Feature:</strong> Additional advanced settings are available in the Pro version.</p>
                    </div>
                  </div>
                )}
              </div>
            </details>
          </div>
        </div>
      )}
      
      {/* Footer */}
      {isExpanded && (
        <div className="flex justify-between p-4 bg-gray-50 dark:bg-gray-700 border-t border-gray-200 dark:border-gray-600">
          <button
            onClick={onCheckForUpdates}
            type="button"
            className="flex items-center space-x-1 px-4 py-2 bg-gray-100 dark:bg-gray-600 text-gray-700 dark:text-gray-200 rounded-md hover:bg-gray-200 dark:hover:bg-gray-500"
          >
            <RefreshCw size={16} />
            <span>Check for Updates</span>
          </button>
          
          <button
            onClick={handleSave}
            type="button"
            className="flex items-center space-x-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
          >
            <Save size={16} />
            <span>Save Settings</span>
          </button>
        </div>
      )}
    </div>
  );
};

export default SettingsPanel;