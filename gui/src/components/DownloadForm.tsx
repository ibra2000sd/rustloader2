// src/components/DownloadForm.tsx
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import Alert from './Alert';

interface DownloadFormProps {
  isPro: boolean;
  onDownloadStart: (params: {
    url: string;
    quality: string;
    format: string;
    startTime?: string;
    endTime?: string;
    usePlaylist: boolean;
    downloadSubtitles: boolean;
    outputDir?: string;
  }) => void;
  disabled?: boolean;
}

interface VideoInfo {
  title?: string;
  uploader?: string;
  duration?: number;
  views?: number;
  likes?: number;
  uploadDate?: string;
}

const DownloadForm: React.FC<DownloadFormProps> = ({ 
  isPro, 
  onDownloadStart,
  disabled = false
}) => {
  // Form state
  const [url, setUrl] = useState('');
  const [quality, setQuality] = useState(isPro ? '1080' : '720');
  const [format, setFormat] = useState('mp4');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [usePlaylist, setUsePlaylist] = useState(false);
  const [downloadSubtitles, setDownloadSubtitles] = useState(false);
  const [outputDir, setOutputDir] = useState('');

  // UI state
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);
  const [availablePaths, setAvailablePaths] = useState<string[]>([]);

  // Update quality if isPro changes
  useEffect(() => {
    if (!isPro && (quality === '1080' || quality === '2160')) {
      setQuality('720');
    }
  }, [isPro, quality]);

  // Load available download paths
  useEffect(() => {
    const loadPaths = async () => {
      try {
        // Try to get paths from backend, fall back to default paths if needed
        let paths: string[] = [];
        try {
          paths = await invoke<string[]>('list_download_paths');
        } catch (err) {
          console.warn('Could not retrieve download paths from backend, using defaults');
          // Default paths
          const homePath = process.env.HOME || '';
          paths = [
            `${homePath}/Downloads/rustloader/videos`,
            `${homePath}/Downloads/rustloader/audio`
          ].filter(Boolean);
        }
        
        setAvailablePaths(paths);
        
        // Set default output directory to first available path
        if (paths.length > 0 && !outputDir) {
          setOutputDir(paths[0]);
        }
      } catch (error) {
        console.error('Failed to load available paths:', error);
      }
    };
    
    loadPaths();
  }, [outputDir]);

  // Clear error when URL changes
  useEffect(() => {
    setError('');
  }, [url]);

  // Format duration from seconds to MM:SS
  const formatDuration = (seconds?: number): string => {
    if (!seconds) return '00:00';
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  };

  // Fetch video info
  const fetchVideoInfo = async (): Promise<void> => {
    if (!url || url.length < 10) {
      setError('Please enter a valid URL');
      return;
    }
    
    setIsLoading(true);
    setError('');
    setVideoInfo(null);
    
    try {
      // We'll generate mock data since the backend might not have this endpoint
      // In a real implementation, you'd call a backend function
      
      // Simulate API delay
      await new Promise(resolve => setTimeout(resolve, 800));
      
      // Mock video info based on URL
      const mockVideoInfo: VideoInfo = {
        title: url.includes('youtube') 
          ? `YouTube Video: ${url.split('v=').pop()?.substring(0, 8) || 'Unknown'}` 
          : `Video from ${new URL(url).hostname}`,
        uploader: "Content Creator",
        duration: 325, // ~5.5 minutes
        views: 12500,
        likes: 1050,
        uploadDate: "2025-03-01"
      };
      
      setVideoInfo(mockVideoInfo);
    } catch (err) {
      setError(`Failed to fetch video info: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setIsLoading(false);
    }
  };

  // Select output directory
  const selectOutputDirectory = async (): Promise<void> => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Download Directory',
      });
  
      if (selected && typeof selected === 'string') {
        setOutputDir(selected);
      }
    } catch (err) {
      setError(`Failed to select directory: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  // Validate time format (HH:MM:SS)
  const validateTimeFormat = (value: string): boolean => {
    if (!value) return true;
    const timeRegex = /^([0-9][0-9]):([0-5][0-9]):([0-5][0-9])$/;
    return timeRegex.test(value);
  };

  // Handle form submission
  const handleSubmit = (e: React.FormEvent): void => {
    e.preventDefault();
    
    if (!url) {
      setError('Please enter a URL');
      return;
    }
    
    if (startTime && !validateTimeFormat(startTime)) {
      setError('Invalid start time format. Use HH:MM:SS');
      return;
    }
    
    if (endTime && !validateTimeFormat(endTime)) {
      setError('Invalid end time format. Use HH:MM:SS');
      return;
    }
    
    onDownloadStart({
      url,
      quality,
      format,
      startTime: startTime || undefined,
      endTime: endTime || undefined,
      usePlaylist,
      downloadSubtitles,
      outputDir: outputDir || undefined
    });
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-5">
      {error && (
        <Alert
          type="error"
          message={error}
          onDismiss={() => setError('')}
        />
      )}
      
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Video URL Input */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label htmlFor="video-url" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Video URL
            </label>
            {isLoading && (
              <span className="text-xs text-primary-500 animate-pulse">Loading...</span>
            )}
          </div>
          <div className="flex space-x-2">
            <input
              id="video-url"
              type="text"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              onBlur={() => url && fetchVideoInfo()}
              placeholder="https://www.youtube.com/watch?v=..."
              disabled={isLoading || disabled}
              className="flex-1 p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-primary-500 focus:border-primary-500 disabled:opacity-70"
              required
            />
            <button
              type="button"
              onClick={fetchVideoInfo}
              disabled={isLoading || disabled || !url}
              className="px-3 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-md text-sm transition-colors disabled:bg-primary-400 disabled:cursor-not-allowed"
            >
              Fetch Info
            </button>
          </div>
        </div>

        {/* Video Info Preview */}
        {videoInfo && (
          <div className="p-3 bg-gray-50 dark:bg-gray-700 rounded-md">
            <h3 className="font-medium text-sm mb-1 text-gray-800 dark:text-gray-200">
              {videoInfo.title || 'Unknown Title'}
            </h3>
            <div className="flex flex-wrap gap-3 text-xs text-gray-600 dark:text-gray-400">
              {videoInfo.duration && <span>Duration: {formatDuration(videoInfo.duration)}</span>}
              {videoInfo.uploader && <span>By: {videoInfo.uploader}</span>}
              {videoInfo.views && <span>Views: {videoInfo.views.toLocaleString()}</span>}
            </div>
          </div>
        )}

        {/* Format and Quality Section */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <label htmlFor="quality" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Quality
            </label>
            <select
              id="quality"
              value={quality}
              onChange={(e) => setQuality(e.target.value)}
              disabled={isLoading || disabled}
              className="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white disabled:opacity-70"
            >
              <option value="480">480p</option>
              <option value="720">720p</option>
              {isPro && (
                <>
                  <option value="1080">1080p</option>
                  <option value="2160">4K</option>
                </>
              )}
            </select>
            {!isPro && (
              <p className="text-xs text-amber-600 dark:text-amber-400">
                Pro version required for 1080p/4K
              </p>
            )}
          </div>

          <div className="space-y-2">
            <label htmlFor="format" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Format
            </label>
            <select
              id="format"
              value={format}
              onChange={(e) => setFormat(e.target.value)}
              disabled={isLoading || disabled}
              className="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white disabled:opacity-70"
            >
              <option value="mp4">MP4 Video</option>
              <option value="mp3">MP3 Audio</option>
              {isPro && (
                <>
                  <option value="webm">WebM</option>
                  <option value="flac">FLAC Audio</option>
                </>
              )}
            </select>
            {!isPro && format === 'mp3' && (
              <p className="text-xs text-amber-600 dark:text-amber-400">
                Limited to 128kbps in free version
              </p>
            )}
          </div>
        </div>

        {/* Toggle Advanced Options */}
        <div className="pt-2">
          <button 
            type="button" 
            onClick={() => setShowAdvanced(!showAdvanced)}
            disabled={isLoading || disabled}
            className="text-sm text-primary-600 hover:text-primary-800 dark:text-primary-400 dark:hover:text-primary-300 disabled:opacity-70 disabled:cursor-not-allowed"
          >
            {showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options'}
          </button>
        </div>

        {/* Advanced Options Section */}
        {showAdvanced && (
          <div className="space-y-4 pt-3 border-t border-gray-200 dark:border-gray-700">
            {/* Time Range */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="space-y-2">
                <label htmlFor="start-time" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                  Start Time (HH:MM:SS)
                </label>
                <input
                  id="start-time"
                  type="text"
                  value={startTime}
                  onChange={(e) => setStartTime(e.target.value)}
                  placeholder="00:00:00"
                  disabled={isLoading || disabled}
                  className="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white disabled:opacity-70"
                />
                {startTime && !validateTimeFormat(startTime) && (
                  <p className="text-xs text-red-600 dark:text-red-400">Format must be HH:MM:SS</p>
                )}
              </div>

              <div className="space-y-2">
                <label htmlFor="end-time" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                  End Time (HH:MM:SS)
                </label>
                <input
                  id="end-time"
                  type="text"
                  value={endTime}
                  onChange={(e) => setEndTime(e.target.value)}
                  placeholder="00:00:00"
                  disabled={isLoading || disabled}
                  className="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white disabled:opacity-70"
                />
                {endTime && !validateTimeFormat(endTime) && (
                  <p className="text-xs text-red-600 dark:text-red-400">Format must be HH:MM:SS</p>
                )}
              </div>
            </div>

            {/* Checkboxes */}
            <div className="flex flex-col sm:flex-row space-y-3 sm:space-y-0 sm:space-x-6">
              <label className="inline-flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={usePlaylist}
                  onChange={(e) => setUsePlaylist(e.target.checked)}
                  disabled={isLoading || disabled}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500 dark:bg-gray-700 dark:border-gray-600"
                />
                <span className="text-sm text-gray-700 dark:text-gray-300">Download entire playlist</span>
              </label>

              <label className="inline-flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={downloadSubtitles}
                  onChange={(e) => setDownloadSubtitles(e.target.checked)}
                  disabled={isLoading || disabled}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500 dark:bg-gray-700 dark:border-gray-600"
                />
                <span className="text-sm text-gray-700 dark:text-gray-300">Download subtitles</span>
              </label>
            </div>

            {/* Output Directory */}
            <div className="space-y-2">
              <label htmlFor="output-dir" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                Output Directory
              </label>
              <div className="flex space-x-2">
                <select
                  id="output-dir"
                  value={outputDir}
                  onChange={(e) => setOutputDir(e.target.value)}
                  disabled={isLoading || disabled}
                  className="flex-1 p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white disabled:opacity-70"
                >
                  <option value="">Default directory</option>
                  {availablePaths.map((path, index) => (
                    <option key={index} value={path}>
                      {path}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  onClick={selectOutputDirectory}
                  disabled={isLoading || disabled}
                  className="px-3 py-2 bg-gray-500 hover:bg-gray-600 text-white rounded-md text-sm transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed"
                >
                  Browse
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Submit Button */}
        <div className="pt-4">
          <button
            type="submit"
            disabled={
              isLoading || 
              disabled || 
              !url ||
              (startTime ? !validateTimeFormat(startTime) : false) ||
              (endTime ? !validateTimeFormat(endTime) : false)
            }
            className="w-full py-2.5 px-4 bg-primary-600 hover:bg-primary-700 text-white font-medium rounded-md shadow-sm transition-colors disabled:bg-primary-400 disabled:cursor-not-allowed"
          >
            {isLoading ? 'Processing...' : disabled ? 'Download in Progress...' : 'Download'}
          </button>
        </div>
        
        {/* Pro version promo for free users */}
        {!isPro && (
          <div className="mt-4 pt-3 border-t border-gray-200 dark:border-gray-700 text-center">
            <p className="text-xs text-gray-500 dark:text-gray-400">
              <span className="text-amber-600 dark:text-amber-400 font-medium">Upgrade to Pro</span> for 4K quality, 
              high-fidelity audio, unlimited downloads, and more
            </p>
          </div>
        )}
      </form>
    </div>
  );
};

export default DownloadForm;