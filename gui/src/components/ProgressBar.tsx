// src/components/ProgressBar.tsx
import React from 'react';

interface ProgressBarProps {
  progress: number;
  fileName?: string;
  fileSize?: number;
  speed?: number;
  timeRemaining?: number;
}

const ProgressBar: React.FC<ProgressBarProps> = ({ 
  progress, 
  fileName = 'Downloading...', 
  fileSize, 
  speed, 
  timeRemaining 
}) => {
  // Ensure progress is between 0 and 100
  const normalizedProgress = Math.min(100, Math.max(0, progress || 0));
  
  // Format bytes in a human-readable format
  const formatBytes = (bytes: number | undefined): string => {
    if (!bytes) return '0 B';
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${sizes[i]}`;
  };
  
  // Format time remaining
  const formatTimeRemaining = (seconds: number | undefined): string => {
    if (!seconds) return 'Calculating...';
    if (seconds < 60) return `${seconds.toFixed(0)}s`;
    if (seconds < 3600) {
      const mins = Math.floor(seconds / 60);
      const secs = Math.floor(seconds % 60);
      return `${mins}m ${secs}s`;
    }
    const hours = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${mins}m`;
  };
  
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
      <div className="flex items-center justify-between mb-1">
        <h3 className="text-sm font-medium truncate max-w-[70%] text-gray-700 dark:text-gray-200">
          {fileName}
        </h3>
        <span className="text-sm font-medium text-primary-600 dark:text-primary-400">
          {normalizedProgress.toFixed(1)}%
        </span>
      </div>
      
      <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5 mb-3">
        <div 
          className="bg-primary-600 dark:bg-primary-500 h-2.5 rounded-full transition-all duration-300"
          style={{ width: `${normalizedProgress}%` }} 
        ></div>
      </div>
      
      <div className="grid grid-cols-2 gap-4 text-xs text-gray-500 dark:text-gray-400">
        <div>
          <span className="font-medium">Size:</span> {formatBytes(fileSize)}
        </div>
        <div>
          <span className="font-medium">Speed:</span> {speed ? `${formatBytes(speed)}/s` : 'Calculating...'}
        </div>
        <div className="col-span-2">
          <span className="font-medium">ETA:</span> {formatTimeRemaining(timeRemaining)}
        </div>
      </div>
      
      {normalizedProgress >= 100 && (
        <div className="mt-2 text-sm text-green-600 dark:text-green-400 font-medium">
          Download complete! Processing file...
        </div>
      )}
    </div>
  );
};

export default ProgressBar;