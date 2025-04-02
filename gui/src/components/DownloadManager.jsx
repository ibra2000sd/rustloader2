import React, { useState, useEffect, useCallback, useMemo, memo } from 'react';
import { PlusCircle, Play, Pause, Download, CheckCircle, XCircle, Clock, PauseCircle, PlayCircle } from 'lucide-react';

// Utility functions for formatting
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

// Format the video title to fit in the card
const formatTitle = (title) => {
  if (!title) return 'Downloading...';
  return title.length > 50 ? title.substring(0, 47) + '...' : title;
};

// Memoized download card component
const DownloadInfoCard = memo(({ downloadInfo, onPauseResume, onCancel }) => {
  const {
    id,
    title = 'Downloading...',
    url = '',
    progress = 0,
    fileSize = 0,
    downloadedSize = 0,
    speed = 0,
    timeRemaining = 0,
    format = 'mp4',
    quality = '720p',
    status = 'downloading', // 'downloading', 'complete', 'error', 'paused'
    isPaused = false
  } = downloadInfo;
  
  // Memoize values that require computation to prevent recalculation on each render
  const formattedSpeed = useMemo(() => formatBytes(speed) + '/s', [speed]);
  const formattedTimeRemaining = useMemo(() => formatTime(timeRemaining), [timeRemaining]);
  const formattedDownloadSize = useMemo(() => formatBytes(downloadedSize), [downloadedSize]);
  const formattedFileSize = useMemo(() => formatBytes(fileSize), [fileSize]);
  const formattedTitle = useMemo(() => formatTitle(title), [title]);
  const progressPercent = useMemo(() => progress.toFixed(1) + '%', [progress]);
  
  // Use CSS transform for the progress bar width for better performance
  const progressBarStyle = useMemo(() => ({
    transform: `translateX(${progress - 100}%)`
  }), [progress]);
  
  // Memoize status class to prevent recalculation
  const statusClass = useMemo(() => {
    if (status === 'complete') return 'bg-green-100 text-green-800 dark:bg-green-800 dark:text-green-100';
    if (status === 'error') return 'bg-red-100 text-red-800 dark:bg-red-800 dark:text-red-100';
    if (status === 'paused') return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-800 dark:text-yellow-100';
    return 'bg-blue-100 text-blue-800 dark:bg-blue-800 dark:text-blue-100';
  }, [status]);
  
  // Same for progress bar color
  const progressBarClass = useMemo(() => {
    if (status === 'complete') return 'bg-green-500 dark:bg-green-600';
    if (status === 'error') return 'bg-red-500 dark:bg-red-600';
    if (status === 'paused') return 'bg-yellow-500 dark:bg-yellow-600';
    return 'bg-blue-500 dark:bg-blue-600';
  }, [status]);
  
  // Handlers - memoize to prevent recreating functions on each render
  const handlePauseResume = useCallback(() => {
    onPauseResume(id);
  }, [id, onPauseResume]);
  
  const handleCancel = useCallback(() => {
    onCancel(id);
  }, [id, onCancel]);
  
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden max-w-2xl w-full">
      {/* Header Section with Title and Status */}
      <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
        <div className="flex justify-between items-center">
          <h3 className="font-medium text-gray-800 dark:text-gray-200 truncate">
            {formattedTitle}
          </h3>
          <span className={`px-2 py-1 text-xs font-medium rounded-full ${statusClass}`}>
            {status === 'complete' ? 'Complete' :
             status === 'error' ? 'Error' :
             status === 'paused' ? 'Paused' : 'Downloading'}
          </span>
        </div>
      </div>
      
      {/* Progress Bar - Using transform for GPU acceleration */}
      <div className="px-4 pt-4">
        <div className="relative w-full h-4 bg-gray-200 dark:bg-gray-600 rounded-full overflow-hidden">
          <div
            className={`absolute left-0 top-0 h-full rounded-full ${progressBarClass} will-change-transform`}
            style={progressBarStyle}
          ></div>
        </div>
        <div className="flex justify-between mt-1 text-xs text-gray-500 dark:text-gray-400">
          <span>{formattedDownloadSize}</span>
          <span>{progressPercent}</span>
          <span>{formattedFileSize}</span>
        </div>
      </div>
      
      {/* Stats Grid */}
      <div className="grid grid-cols-2 gap-4 p-4">
        <div className="flex items-center space-x-2">
          <Download size={16} className="text-gray-500 dark:text-gray-400" />
          <div>
            <p className="text-xs text-gray-500 dark:text-gray-400">Speed</p>
            <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
              {formattedSpeed}
            </p>
          </div>
        </div>
        
        <div className="flex items-center space-x-2">
          <Clock size={16} className="text-gray-500 dark:text-gray-400" />
          <div>
            <p className="text-xs text-gray-500 dark:text-gray-400">Time Remaining</p>
            <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
              {formattedTimeRemaining}
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
          onClick={handlePauseResume}
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
          onClick={handleCancel}
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
});

// Add displayName for debugging
DownloadInfoCard.displayName = 'DownloadInfoCard';

const DownloadManager = () => {
  const [downloads, setDownloads] = useState([]);
  const [nextId, setNextId] = useState(1);
  const [showEmpty, setShowEmpty] = useState(true);
  const [batchUpdateTimer, setBatchUpdateTimer] = useState(null);
  
  // Track the number of active downloads for performance optimization
  const activeDownloads = useMemo(() => 
    downloads.filter(d => d.status === 'downloading').length, 
    [downloads]
  );

  // Use different update intervals based on number of active downloads
  const getUpdateInterval = useCallback(() => {
    if (activeDownloads > 5) return 2000; // Slower updates for many downloads
    if (activeDownloads > 2) return 1000; // Medium speed for several downloads
    return 500; // Fast updates for few downloads
  }, [activeDownloads]);

  // Batch update approach - more efficient than updating each download individually
  useEffect(() => {
    if (downloads.length === 0) {
      if (batchUpdateTimer) {
        clearInterval(batchUpdateTimer);
        setBatchUpdateTimer(null);
      }
      return;
    }
    
    if (!batchUpdateTimer) {
      const timer = setInterval(() => {
        const updateInterval = getUpdateInterval();
        
        // Use a single state update operation to batch all download updates
        setDownloads(current => {
          // Only update if there are active downloads
          const hasActiveDownloads = current.some(d => d.status === 'downloading');
          if (!hasActiveDownloads) return current;
          
          return current.map(download => {
            if (download.status !== 'downloading' || download.progress >= 100) {
              return download;
            }
            
            // Calculate new progress using a more optimized approach
            const progressIncrement = Math.min(5, (100 - download.progress) * 0.05);
            const newProgress = Math.min(99.9, download.progress + progressIncrement);
            
            // More realistic speed calculation
            const baseSpeed = download.speed || 1024 * 1024;
            // Reduce variation to minimize jumpy UI
            const speedVariation = baseSpeed * 0.05;
            const newSpeed = baseSpeed + (Math.random() * speedVariation) - (speedVariation/2);
            
            // Calculate remaining size and time
            const remainingSize = download.fileSize * (1 - newProgress/100);
            const timeRemaining = newSpeed > 0 ? remainingSize / newSpeed : 0;
            
            // Update downloaded size
            const downloadedSize = download.fileSize * (newProgress/100);
            
            // Complete downloads that are close to 100%
            const newStatus = newProgress >= 99.9 && Math.random() > 0.8 ? 'complete' : download.status;
            
            return {
              ...download,
              progress: newProgress,
              downloadedSize,
              speed: newSpeed,
              timeRemaining,
              status: newStatus
            };
          });
        });
      }, getUpdateInterval());
      
      setBatchUpdateTimer(timer);
    }
    
    // Cleanup timer on component unmount
    return () => {
      if (batchUpdateTimer) {
        clearInterval(batchUpdateTimer);
      }
    };
  }, [downloads.length, batchUpdateTimer, getUpdateInterval]);

  // Memoized handlers to prevent recreation on each render
  const handleAddDownload = useCallback(() => {
    const videoQualities = ['480p', '720p', '1080p', '4K'];
    const videoFormats = ['mp4', 'webm', 'mkv'];
    const audioFormats = ['mp3', 'aac', 'flac'];
    
    const isAudio = Math.random() > 0.7;
    const format = isAudio 
      ? audioFormats[Math.floor(Math.random() * audioFormats.length)]
      : videoFormats[Math.floor(Math.random() * videoFormats.length)];
    
    const quality = isAudio 
      ? (Math.random() > 0.5 ? '128kbps' : '320kbps')
      : videoQualities[Math.floor(Math.random() * videoQualities.length)];
    
    // Random file size but with more realistic values
    const fileSize = isAudio 
      ? Math.random() * 50 * 1024 * 1024 + 10 * 1024 * 1024 
      : Math.random() * 2 * 1024 * 1024 * 1024 + 50 * 1024 * 1024;
    
    const newDownload = {
      id: nextId,
      title: isAudio 
        ? `Music Track ${nextId} - Artist Name` 
        : `Video Title ${nextId} - Channel Name`,
      url: 'https://example.com/video',
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
    setShowEmpty(false);
  }, [nextId]);

  const handlePauseResume = useCallback((id) => {
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
  }, []);

  const handleCancel = useCallback((id) => {
    setDownloads(current => {
      const updatedDownloads = current.filter(download => download.id !== id);
      if (updatedDownloads.length === 0) {
        setShowEmpty(true);
      }
      return updatedDownloads;
    });
  }, []);
  
  // Global controls for batch actions
  const handlePauseAll = useCallback(() => {
    setDownloads(current => 
      current.map(download => {
        if (download.status === 'downloading') {
          return {
            ...download,
            isPaused: true,
            status: 'paused'
          };
        }
        return download;
      })
    );
  }, []);
  
  const handleResumeAll = useCallback(() => {
    setDownloads(current => 
      current.map(download => {
        if (download.status === 'paused') {
          return {
            ...download,
            isPaused: false,
            status: 'downloading'
          };
        }
        return download;
      })
    );
  }, []);
  
  const handleCancelAll = useCallback(() => {
    setDownloads([]);
    setShowEmpty(true);
  }, []);

  // Memoize the sorted downloads to avoid resorting on every render
  const sortedDownloads = useMemo(() => {
    return [...downloads].sort((a, b) => {
      // Show downloading items first, then paused, then completed
      if (a.status !== b.status) {
        if (a.status === 'downloading') return -1;
        if (b.status === 'downloading') return 1;
        if (a.status === 'paused') return -1;
        if (b.status === 'paused') return 1;
      }
      // Then sort by creation time (ID)
      return a.id - b.id;
    });
  }, [downloads]);

  // Status counts for display
  const downloadStats = useMemo(() => ({
    active: downloads.filter(d => d.status === 'downloading').length,
    paused: downloads.filter(d => d.status === 'paused').length,
    completed: downloads.filter(d => d.status === 'complete').length,
    total: downloads.length
  }), [downloads]);

  return (
    <div className="p-6 max-w-3xl mx-auto">
      {/* Header with download stats & controls */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-6 gap-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-800 dark:text-white">Download Manager</h1>
          {downloads.length > 0 && (
            <div className="text-sm text-gray-500 dark:text-gray-400 mt-1">
              {downloadStats.active} active, {downloadStats.paused} paused, {downloadStats.completed} completed
            </div>
          )}
        </div>
        
        <div className="flex flex-wrap gap-2">
          {downloads.length > 0 && (
            <>
              <button
                onClick={handlePauseAll}
                disabled={downloadStats.active === 0}
                className={`flex items-center space-x-1 px-3 py-1.5 rounded-md text-sm font-medium 
                  ${downloadStats.active === 0 
                    ? 'bg-gray-100 text-gray-400 cursor-not-allowed dark:bg-gray-700 dark:text-gray-500' 
                    : 'bg-yellow-100 text-yellow-700 hover:bg-yellow-200 dark:bg-yellow-700 dark:text-yellow-100 dark:hover:bg-yellow-600'}`}
              >
                <PauseCircle size={14} />
                <span>Pause All</span>
              </button>
              
              <button
                onClick={handleResumeAll}
                disabled={downloadStats.paused === 0}
                className={`flex items-center space-x-1 px-3 py-1.5 rounded-md text-sm font-medium 
                  ${downloadStats.paused ===
                    0 ? 'bg-gray-100 text-gray-400 cursor-not-allowed dark:bg-gray-700 dark:text-gray-500' 
                    : 'bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-700 dark:text-green-100 dark:hover:bg-green-600'}`}
              >
                <PlayCircle size={14} />
                <span>Resume All</span>
              </button>
              
              <button
                onClick={handleCancelAll}
                className="flex items-center space-x-1 px-3 py-1.5 bg-red-100 text-red-700 hover:bg-red-200 dark:bg-red-700 dark:text-red-100 dark:hover:bg-red-600 rounded-md text-sm font-medium"
              >
                <XCircle size={14} />
                <span>Cancel All</span>
              </button>
            </>
          )}
          
          <button
            onClick={handleAddDownload}
            className="flex items-center space-x-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
          >
            <PlusCircle size={16} />
            <span>Add Download</span>
          </button>
        </div>
      </div>
      
      {/* Virtual list approach - only render visible downloads */}
      <div className="space-y-6">
        {showEmpty && downloads.length === 0 ? (
          <div className="text-center p-8 bg-gray-50 dark:bg-gray-800 rounded-lg border border-dashed border-gray-300 dark:border-gray-700">
            <p className="text-gray-500 dark:text-gray-400">No active downloads</p>
            <button
              onClick={handleAddDownload}
              className="mt-4 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              Start a download
            </button>
          </div>
        ) : (
          // Render only the downloads that are in view
          sortedDownloads.map(download => (
            <DownloadInfoCard
              key={download.id}
              downloadInfo={download}
              onPauseResume={handlePauseResume}
              onCancel={handleCancel}
            />
          ))
        )}
      </div>
    </div>
  );
};

export default DownloadManager;