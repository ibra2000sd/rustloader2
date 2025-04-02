// src/components/ProgressBar.tsx
import React, { memo, useMemo } from 'react';
import './ProgressBar.css';

interface ProgressBarProps {
  progress: number;
  fileName?: string;
  fileSize?: number;
  speed?: number;
  timeRemaining?: number;
  status?: 'downloading' | 'paused' | 'complete' | 'error';
}

const formatBytes = (bytes: number | undefined): string => {
  if (!bytes) return '0 B';
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${sizes[i]}`;
};

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

// Use memo to prevent unnecessary re-renders
const ProgressBar: React.FC<ProgressBarProps> = memo(({ 
  progress, 
  fileName = 'Downloading...', 
  fileSize, 
  speed, 
  timeRemaining,
  status = 'downloading'
}) => {
  // Ensure progress is between 0 and 100
  const normalizedProgress = Math.min(100, Math.max(0, progress || 0));
  
  // Memoize formatted values to prevent recalculation on every render
  const formattedFileSize = useMemo(() => formatBytes(fileSize), [fileSize]);
  const formattedSpeed = useMemo(() => speed ? `${formatBytes(speed)}/s` : 'Calculating...', [speed]);
  const formattedETA = useMemo(() => formatTimeRemaining(timeRemaining), [timeRemaining]);
  
  // Determine progress bar color based on status
  const progressBarClass = useMemo(() => {
    switch (status) {
      case 'paused': return 'progress-bar-paused';
      case 'complete': return 'progress-bar-complete';
      case 'error': return 'progress-bar-error';
      default: return 'progress-bar-active';
    }
  }, [status]);
  
  // Use CSS transform for smoother progress bar animation
  // This offloads animation to GPU instead of CPU
  const barStyle = useMemo(() => ({
    transform: `translateX(${normalizedProgress - 100}%)`,
  }), [normalizedProgress]);

  return (
    <div className="progress-container" style={{ contain: 'layout paint' }}>
      <div className="progress-header">
        <h3 className="progress-title">{fileName}</h3>
        <span className="progress-percentage">
          {normalizedProgress.toFixed(0)}%
        </span>
      </div>
      
      <div className="progress-track">
        <div 
          className={`progress-bar ${progressBarClass}`}
          style={barStyle}
        ></div>
      </div>
      
      <div className="progress-stats">
        <div className="progress-stat">
          <span className="progress-label">Size:</span> {formattedFileSize}
        </div>
        <div className="progress-stat">
          <span className="progress-label">Speed:</span> {formattedSpeed}
        </div>
        <div className="progress-stat progress-eta">
          <span className="progress-label">ETA:</span> {formattedETA}
        </div>
      </div>
      
      {normalizedProgress >= 100 && (
        <div className="progress-complete-message">
          Download complete!
        </div>
      )}
    </div>
  );
});

// Add display name for debugging
ProgressBar.displayName = 'ProgressBar';

export default ProgressBar;