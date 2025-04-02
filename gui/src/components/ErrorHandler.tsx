import React, { useState } from 'react';
import { AlertTriangle, XCircle, RefreshCw, ChevronDown, ChevronUp, Copy } from 'lucide-react';

interface ErrorHandlerProps {
  error: string;
  suggestion?: string;
  actions?: {
    label: string;
    action: () => void;
    primary?: boolean;
  }[];
  technical?: string;
  onDismiss?: () => void;
}

// Map common errors to user-friendly messages and suggestions
const getErrorInfo = (errorText: string) => {
  // Network errors
  if (errorText.includes('network') || errorText.includes('connect') || errorText.includes('timeout')) {
    return {
      message: 'Connection problem detected',
      suggestion: 'Check your internet connection and try again. If the problem persists, the service might be temporarily unavailable.',
      actions: [
        { label: 'Retry', action: () => window.location.reload(), primary: true },
        { label: 'Work Offline', action: () => console.log('Switch to offline mode') }
      ]
    };
  }
  
  // Format errors
  if (errorText.includes('format') || errorText.includes('quality') || errorText.includes('resolution')) {
    return {
      message: 'Format or quality unavailable',
      suggestion: 'The selected format or quality is not available for this media. Try selecting a different quality or format.',
      actions: [
        { label: 'Try Different Format', action: () => console.log('Change format'), primary: true }
      ]
    };
  }
  
  // Permission errors
  if (errorText.includes('permission') || errorText.includes('access') || errorText.includes('denied')) {
    return {
      message: 'Permission error',
      suggestion: 'Rustloader doesn\'t have permission to access the required resources. Check your system permissions.',
      actions: [
        { label: 'Open Settings', action: () => console.log('Open settings'), primary: true }
      ]
    };
  }
  
  // URL errors
  if (errorText.includes('url') || errorText.includes('link') || errorText.includes('not found')) {
    return {
      message: 'Invalid or unsupported URL',
      suggestion: 'The URL you entered may be invalid, unsupported, or the content might have been removed.',
      actions: [
        { label: 'Check Supported Sites', action: () => console.log('Show supported sites') }
      ]
    };
  }
  
  // Dependency errors
  if (errorText.includes('dependency') || errorText.includes('missing') || errorText.includes('command not found')) {
    return {
      message: 'Missing system dependency',
      suggestion: 'A required system component is missing. Rustloader will help you install it.',
      actions: [
        { label: 'Install Dependencies', action: () => console.log('Install dependencies'), primary: true }
      ]
    };
  }
  
  // Default fallback
  return {
    message: 'An error occurred',
    suggestion: 'Something went wrong. Try again or check the logs for more information.',
    actions: [
      { label: 'Retry', action: () => window.location.reload() }
    ]
  };
};

const ErrorHandler: React.FC<ErrorHandlerProps> = ({ 
  error, 
  suggestion: customSuggestion, 
  actions: customActions,
  technical,
  onDismiss 
}) => {
  const [showDetails, setShowDetails] = useState(false);
  
  // Get error information either from custom props or from our error mapping
  const errorInfo = getErrorInfo(error);
  const displayMessage = errorInfo.message;
  const displaySuggestion = customSuggestion || errorInfo.suggestion;
  const displayActions = customActions || errorInfo.actions;
  
  // Handle copying technical details to clipboard
  const copyTechnicalDetails = () => {
    if (technical) {
      navigator.clipboard.writeText(technical)
        .then(() => alert('Technical details copied to clipboard'))
        .catch(err => console.error('Failed to copy text:', err));
    }
  };

  return (
    <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 mb-4 animate-fadeIn">
      <div className="flex items-start">
        <div className="flex-shrink-0">
          <AlertTriangle className="h-5 w-5 text-red-500 dark:text-red-400" />
        </div>
        
        <div className="ml-3 flex-1">
          <h3 className="text-sm font-medium text-red-800 dark:text-red-300">
            {displayMessage}
          </h3>
          
          <div className="mt-2 text-sm text-red-700 dark:text-red-200">
            <p>{displaySuggestion}</p>
          </div>
          
          {(technical || error) && (
            <div className="mt-2">
              <button
                type="button"
                onClick={() => setShowDetails(!showDetails)}
                className="flex items-center text-xs text-red-600 dark:text-red-300 hover:text-red-800 dark:hover:text-red-100"
              >
                {showDetails ? (
                  <>
                    <ChevronUp size={14} className="mr-1" />
                    Hide technical details
                  </>
                ) : (
                  <>
                    <ChevronDown size={14} className="mr-1" />
                    Show technical details
                  </>
                )}
              </button>
              
              {showDetails && (
                <div className="mt-2 p-2 bg-red-100 dark:bg-red-900/30 rounded text-xs font-mono text-red-800 dark:text-red-200 relative">
                  <button
                    onClick={copyTechnicalDetails}
                    className="absolute top-1 right-1 p-1 hover:bg-red-200 dark:hover:bg-red-800 rounded"
                    title="Copy to clipboard"
                  >
                    <Copy size={12} />
                  </button>
                  <pre className="whitespace-pre-wrap break-words overflow-x-auto max-h-40 overflow-y-auto">
                    {technical || error}
                  </pre>
                </div>
              )}
            </div>
          )}
          
          {displayActions && displayActions.length > 0 && (
            <div className="mt-4 flex flex-wrap gap-2">
              {displayActions.map((action, index) => (
                <button
                  key={index}
                  type="button"
                  onClick={action.action}
                  className={`inline-flex items-center px-3 py-1.5 border text-xs font-medium rounded-md shadow-sm ${
                    action.primary
                      ? 'text-white bg-red-600 hover:bg-red-700 border-red-600 dark:border-red-700'
                      : 'text-red-700 bg-white hover:bg-red-50 border-red-300 dark:text-red-200 dark:bg-red-900/40 dark:border-red-700 dark:hover:bg-red-900/60'
                  }`}
                >
                  {action.label === 'Retry' && <RefreshCw size={12} className="mr-1" />}
                  {action.label}
                </button>
              ))}
              
              {onDismiss && (
                <button
                  type="button"
                  onClick={onDismiss}
                  className="inline-flex items-center px-3 py-1.5 border border-transparent text-xs font-medium rounded-md text-red-600 dark:text-red-300 hover:underline"
                >
                  <XCircle size={12} className="mr-1" />
                  Dismiss
                </button>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default ErrorHandler;