import React, { useState, useEffect } from 'react';
import { Play, Pause, Download, CheckCircle, XCircle, Clock } from 'lucide-react';

const formatBytes = (bytes, decimals = 2) => {
  if (bytes === 0) return '0 Bytes';
  
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
  
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
};

const formatTime = (seconds) => {
  if (!seconds) return '--:--';
  
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60);
  
  return `${String(minutes).padStart(2, '0')}:${String(remainingSeconds).padStart(2, '0')}`;
};

const DownloadInfoCard = ({ 
  downloadInfo = {}, 
  isPaused = false,
  onPauseResume = () => {},
  onCancel = () => {}
}) => {
  const {
    title = 'Downloading...',
    url = '',
    progress = 0,
    fileSize = 0,
    downloadedSize = 0,
    speed = 0,
    timeRemaining = 0,
    format = 'mp4',
    quality = '720p',
    status = 'downloading' // 'downloading', 'complete', 'error', 'paused'
  } = downloadInfo;
  
  const [isPlaying, setIsPlaying] = useState(false);
  
  // Play animation when download completes
  useEffect(() => {
    if (status === 'complete') {
      setIsPlaying(true);
      const timer = setTimeout(() => setIsPlaying(false), 5000);
      return () => clearTimeout(timer);
    }
  }, [status]);
  
  // Format the video title to fit in the card
  const formatTitle = (title) => {
    if (!title) return 'Downloading...';
    return title.length > 50 ? title.substring(0, 47) + '...' : title;
  };
  
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden max-w-2xl w-full">
      {/* Header Section with Title and Status */}
      <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
        <div className="flex justify-between items-center">
          <h3 className="font-medium text-gray-800 dark:text-gray-200 truncate">
            {formatTitle(title)}
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
           />
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

export default DownloadInfoCard;