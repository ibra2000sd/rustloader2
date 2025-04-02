import React, { useState, useRef, useEffect } from 'react';
import { HelpCircle } from 'lucide-react';

interface TooltipProps {
  content: React.ReactNode;
  children?: React.ReactNode;
  position?: 'top' | 'bottom' | 'left' | 'right';
  icon?: boolean;
  className?: string;
  width?: 'auto' | 'wide' | 'narrow';
  delay?: number;
}

const Tooltip: React.FC<TooltipProps> = ({
  content,
  children,
  position = 'top',
  icon = false,
  className = '',
  width = 'auto',
  delay = 300
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLDivElement>(null);

  // Calculate width based on the width prop
  const widthClass = width === 'wide' 
    ? 'w-64' 
    : width === 'narrow' 
      ? 'w-40' 
      : 'w-auto max-w-xs';

  // Position classes
  const positionClasses = {
    top: 'bottom-full left-1/2 transform -translate-x-1/2 mb-2',
    bottom: 'top-full left-1/2 transform -translate-x-1/2 mt-2',
    left: 'right-full top-1/2 transform -translate-y-1/2 mr-2',
    right: 'left-full top-1/2 transform -translate-y-1/2 ml-2'
  };

  // Arrow classes
  const arrowClasses = {
    top: 'top-full left-1/2 transform -translate-x-1/2 border-t-gray-800 dark:border-t-gray-700 border-l-transparent border-r-transparent border-b-transparent',
    bottom: 'bottom-full left-1/2 transform -translate-x-1/2 border-b-gray-800 dark:border-b-gray-700 border-l-transparent border-r-transparent border-t-transparent',
    left: 'left-full top-1/2 transform -translate-y-1/2 border-l-gray-800 dark:border-l-gray-700 border-t-transparent border-b-transparent border-r-transparent',
    right: 'right-full top-1/2 transform -translate-y-1/2 border-r-gray-800 dark:border-r-gray-700 border-t-transparent border-b-transparent border-l-transparent'
  };

  // Handle mouse enter, start the delay timer
  const handleMouseEnter = () => {
    setIsHovered(true);
    clearTimeout(timeoutRef.current!);
    
    timeoutRef.current = setTimeout(() => {
      setIsVisible(true);
    }, delay);
  };

  // Handle mouse leave, clear the timer
  const handleMouseLeave = () => {
    setIsHovered(false);
    clearTimeout(timeoutRef.current!);
    
    // Add a small delay before hiding to prevent flickering
    timeoutRef.current = setTimeout(() => {
      if (!isHovered) {
        setIsVisible(false);
      }
    }, 100);
  };

  // Clean up timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  return (
    <div className="relative inline-block" ref={triggerRef}>
      <div
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onFocus={handleMouseEnter}
        onBlur={handleMouseLeave}
        className={`inline-flex ${icon ? 'cursor-help' : 'cursor-default'} ${className}`}
        tabIndex={0}
      >
        {icon ? (
          <HelpCircle
            size={16}
            className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          />
        ) : (
          children
        )}
      </div>

      {isVisible && (
        <div
          ref={tooltipRef}
          className={`absolute z-50 ${positionClasses[position]} ${widthClass}`}
          role="tooltip"
          onMouseEnter={() => setIsHovered(true)}
          onMouseLeave={handleMouseLeave}
        >
          {/* Arrow */}
          <div
            className={`absolute w-0 h-0 border-4 ${arrowClasses[position]}`}
          />
          
          {/* Content */}
          <div className="bg-gray-800 dark:bg-gray-700 text-white p-2 rounded shadow-lg text-sm">
            {content}
          </div>
        </div>
      )}
    </div>
  );
};

export default Tooltip;