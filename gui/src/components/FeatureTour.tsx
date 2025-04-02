import React, { useState, useEffect, useRef } from 'react';
import { Store } from '@tauri-apps/plugin-store';
import { X, ChevronRight, ChevronLeft } from 'lucide-react';

interface FeatureTourProps {
  onComplete: () => void;
  onSkip: () => void;
  featureKey?: string;
}

interface TourStep {
  target: string;
  content: string;
  title: string;
  position: 'top' | 'right' | 'bottom' | 'left';
}

// Tour configuration for different feature tours
const tourConfigs: Record<string, TourStep[]> = {
  // Main app tour highlighting key UI elements
  'main': [
    {
      target: '#url-input-field',
      title: 'URL Input',
      content: 'Paste a video URL here to start downloading.',
      position: 'bottom'
    },
    {
      target: '#format-selector',
      title: 'Format Selection',
      content: 'Choose your preferred download format (MP4, MP3, etc.).',
      position: 'right'
    },
    {
      target: '#quality-selector',
      title: 'Quality Options',
      content: 'Select the quality for your download.',
      position: 'left'
    },
    {
      target: '#download-button',
      title: 'Start Download',
      content: 'Click here to begin the download process.',
      position: 'top'
    },
    {
      target: '#downloads-list',
      title: 'Downloads List',
      content: 'All your downloads appear here with progress information.',
      position: 'top'
    }
  ],
  
  // Settings tour
  'settings': [
    {
      target: '#download-location-setting',
      title: 'Download Location',
      content: 'Set where your downloads will be saved.',
      position: 'bottom'
    },
    {
      target: '#notification-setting',
      title: 'Notifications',
      content: 'Enable desktop notifications for completed downloads.',
      position: 'right'
    },
    {
      target: '#concurrent-downloads-setting',
      title: 'Concurrent Downloads',
      content: 'Adjust how many downloads can run at the same time.',
      position: 'left'
    }
  ],
  
  // Advanced features tour
  'advanced': [
    {
      target: '#trim-video-section',
      title: 'Video Trimming',
      content: 'Cut videos to specific start and end times.',
      position: 'bottom'
    },
    {
      target: '#subtitles-option',
      title: 'Subtitle Options',
      content: 'Download or generate subtitles for your videos.',
      position: 'right'
    },
    {
      target: '#playlist-settings',
      title: 'Playlist Settings',
      content: 'Configure how playlists are downloaded.',
      position: 'top'
    }
  ]
};

const FeatureTour: React.FC<FeatureTourProps> = ({
  onComplete,
  onSkip,
  featureKey = 'main'
}) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const [arrowPosition, setArrowPosition] = useState({ top: 0, left: 0 });
  const [isVisible, setIsVisible] = useState(false);
  const tooltipRef = useRef<HTMLDivElement>(null);
  
  // Get the tour steps based on feature key
  const steps = tourConfigs[featureKey] || tourConfigs.main;
  const currentTourStep = steps[currentStep];
  
  // Position the tooltip next to the target element
  const positionTooltip = () => {
    const targetElement = document.querySelector(currentTourStep.target);
    
    if (!targetElement || !tooltipRef.current) {
      // If target not found, skip this step
      if (currentStep < steps.length - 1) {
        setCurrentStep(currentStep + 1);
      } else {
        onComplete();
      }
      return;
    }
    
    const targetRect = targetElement.getBoundingClientRect();
    const tooltipRect = tooltipRef.current.getBoundingClientRect();
    
    // Add highlight to the target element
    targetElement.classList.add('feature-tour-highlight');
    
    // Calculate position based on specified direction
    let newPosition = { top: 0, left: 0 };
    let newArrowPosition = { top: 0, left: 0 };
    
    switch (currentTourStep.position) {
      case 'top':
        newPosition = {
          top: targetRect.top - tooltipRect.height - 12,
          left: targetRect.left + (targetRect.width / 2) - (tooltipRect.width / 2)
        };
        newArrowPosition = {
          top: tooltipRect.height,
          left: tooltipRect.width / 2
        };
        break;
      case 'right':
        newPosition = {
          top: targetRect.top + (targetRect.height / 2) - (tooltipRect.height / 2),
          left: targetRect.right + 12
        };
        newArrowPosition = {
          top: tooltipRect.height / 2,
          left: -6
        };
        break;
      case 'bottom':
        newPosition = {
          top: targetRect.bottom + 12,
          left: targetRect.left + (targetRect.width / 2) - (tooltipRect.width / 2)
        };
        newArrowPosition = {
          top: -6,
          left: tooltipRect.width / 2
        };
        break;
      case 'left':
        newPosition = {
          top: targetRect.top + (targetRect.height / 2) - (tooltipRect.height / 2),
          left: targetRect.left - tooltipRect.width - 12
        };
        newArrowPosition = {
          top: tooltipRect.height / 2,
          left: tooltipRect.width
        };
        break;
    }
    
    // Keep tooltip within viewport
    if (newPosition.left < 0) newPosition.left = 10;
    if (newPosition.top < 0) newPosition.top = 10;
    if (newPosition.left + tooltipRect.width > window.innerWidth) {
      newPosition.left = window.innerWidth - tooltipRect.width - 10;
    }
    if (newPosition.top + tooltipRect.height > window.innerHeight) {
      newPosition.top = window.innerHeight - tooltipRect.height - 10;
    }
    
    setPosition(newPosition);
    setArrowPosition(newArrowPosition);
    setDimensions({
      width: targetRect.width,
      height: targetRect.height
    });
    setIsVisible(true);
  };
  
  // Handle next/previous navigation
  const nextStep = () => {
    // Remove highlight from current target
    const currentTarget = document.querySelector(currentTourStep.target);
    if (currentTarget) {
      currentTarget.classList.remove('feature-tour-highlight');
    }
    
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
      setIsVisible(false);
    } else {
      handleComplete();
    }
  };
  
  const prevStep = () => {
    // Remove highlight from current target
    const currentTarget = document.querySelector(currentTourStep.target);
    if (currentTarget) {
      currentTarget.classList.remove('feature-tour-highlight');
    }
    
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
      setIsVisible(false);
    }
  };
  
  // Save completion state and notify parent
  const handleComplete = async () => {
    // Remove highlight from current target
    const currentTarget = document.querySelector(currentTourStep.target);
    if (currentTarget) {
      currentTarget.classList.remove('feature-tour-highlight');
    }
    
    try {
      const store = new Store('preferences.dat');
      await store.set(`completedTour.${featureKey}`, true);
      await store.save();
    } catch (error) {
      console.error('Failed to save tour completion state:', error);
    }
    
    onComplete();
  };
  
  // Handle skip
  const handleSkip = async () => {
    // Remove highlight from current target
    const currentTarget = document.querySelector(currentTourStep.target);
    if (currentTarget) {
      currentTarget.classList.remove('feature-tour-highlight');
    }
    
    try {
      const store = new Store('preferences.dat');
      await store.set(`completedTour.${featureKey}`, true);
      await store.save();
    } catch (error) {
      console.error('Failed to save tour skip state:', error);
    }
    
    onSkip();
  };
  
  // Position tooltip when step changes
  useEffect(() => {
    const timeout = setTimeout(() => {
      positionTooltip();
    }, 200);
    
    return () => {
      clearTimeout(timeout);
    };
  }, [currentStep]);
  
  // Add global styles for the highlight effect
  useEffect(() => {
    const styleElement = document.createElement('style');
    styleElement.textContent = `
      .feature-tour-highlight {
        position: relative;
        z-index: 10;
        box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.5);
        border-radius: 4px;
        animation: pulse 1.5s infinite;
      }
      
      @keyframes pulse {
        0% {
          box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.5);
        }
        70% {
          box-shadow: 0 0 0 8px rgba(59, 130, 246, 0);
        }
        100% {
          box-shadow: 0 0 0 0 rgba(59, 130, 246, 0);
        }
      }
    `;
    document.head.appendChild(styleElement);
    
    return () => {
      document.head.removeChild(styleElement);
      
      // Cleanup any remaining highlights
      document.querySelectorAll('.feature-tour-highlight').forEach(el => {
        el.classList.remove('feature-tour-highlight');
      });
    };
  }, []);
  
  // Arrow styles based on position
  const arrowStyles: Record<string, React.CSSProperties> = {
    top: {
      borderBottom: '8px solid transparent',
      borderTop: '8px solid transparent',
      borderLeft: '8px solid white',
      bottom: '-8px'
    },
    right: {
      borderBottom: '8px solid transparent',
      borderTop: '8px solid transparent',
      borderRight: '8px solid white',
      left: '-8px'
    },
    bottom: {
      borderBottom: '8px solid transparent',
      borderTop: '8px solid white',
      borderLeft: '8px solid transparent',
      borderRight: '8px solid transparent',
      top: '-8px'
    },
    left: {
      borderBottom: '8px solid transparent',
      borderTop: '8px solid transparent',
      borderLeft: '8px solid white',
      right: '-8px'
    }
  };
  
  // In dark mode, adjust the arrow color
  if (document.documentElement.classList.contains('dark')) {
    Object.keys(arrowStyles).forEach(key => {
      const style = arrowStyles[key];
      if (style.borderTop === '8px solid white') {
        style.borderTop = '8px solid #1f2937'; // dark mode background
      }
      if (style.borderRight === '8px solid white') {
        style.borderRight = '8px solid #1f2937';
      }
      if (style.borderBottom === '8px solid white') {
        style.borderBottom = '8px solid #1f2937';
      }
      if (style.borderLeft === '8px solid white') {
        style.borderLeft = '8px solid #1f2937';
      }
    });
  }
  
  if (!isVisible) return null;
  
  return (
    <div 
      ref={tooltipRef}
      className="fixed z-50 bg-white dark:bg-gray-800 rounded-lg shadow-xl border border-gray-200 dark:border-gray-700 w-64"
      style={{
        top: `${position.top}px`,
        left: `${position.left}px`,
        transition: 'all 0.3s ease'
      }}
    >
      {/* Arrow */}
      <div 
        className="absolute w-0 h-0"
        style={{
          top: `${arrowPosition.top}px`,
          left: `${arrowPosition.left}px`,
          ...arrowStyles[currentTourStep.position]
        }}
      />
      
      {/* Close button */}
      <button
        onClick={handleSkip}
        className="absolute top-2 right-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
      >
        <X size={16} />
      </button>
      
      {/* Content */}
      <div className="p-4">
        <h4 className="text-sm font-bold text-gray-800 dark:text-gray-100 mb-1">
          {currentTourStep.title}
        </h4>
        <p className="text-xs text-gray-600 dark:text-gray-300">
          {currentTourStep.content}
        </p>
      </div>
      
      {/* Navigation */}
      <div className="flex justify-between items-center p-2 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750 rounded-b-lg">
        <div className="text-xs text-gray-500 dark:text-gray-400">
          {currentStep + 1} of {steps.length}
        </div>
        <div className="flex space-x-2">
          {currentStep > 0 && (
            <button
              onClick={prevStep}
              className="p-1 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded"
            >
              <ChevronLeft size={16} />
            </button>
          )}
          <button
            onClick={nextStep}
            className="p-1 text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-900/30 rounded"
          >
            {currentStep === steps.length - 1 ? (
              'Finish'
            ) : (
              <ChevronRight size={16} />
            )}
          </button>
        </div>
      </div>
    </div>
  );
};

export default FeatureTour;