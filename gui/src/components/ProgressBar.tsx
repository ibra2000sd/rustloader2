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
  
  // Get color based on progress
  const getProgressColor = (): string => {
    if (normalizedProgress < 30) return 'bg-blue-500';
    if (normalizedProgress < 70) return 'bg-green-500';
    return 'bg-yellow-500';
  };
  
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      {/* File name and progress percentage */}
      <div className="flex justify-between items-center mb-2">
        <div className="truncate max-w-[70%] text-sm font-medium text-gray-800 dark:text-gray-200">
          {fileName}
        </div>
        <div className="text-sm font-medium text-blue-600 dark:text-blue-400">
          {normalizedProgress.toFixed(1)}%
        </div>
      </div>
      
      {/* Progress bar */}
      <div className="h-2.5 bg-gray-200 dark:bg-gray-700 rounded-full mb-3">
        <div 
          className={`h-2.5 rounded-full ${getProgressColor()}`} 
          style={{ width: `${normalizedProgress}%` }}
        ></div>
      </div>
      
      {/* Additional details */}
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
        <div className="mt-3 text-sm text-green-600 dark:text-green-400 font-medium">
          Download complete! Processing file...
        </div>
      )}
    </div>
  );
};

export default ProgressBar;