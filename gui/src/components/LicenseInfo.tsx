import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface LicenseActivationProps {
  isProVersion: boolean;
  onActivationComplete: (success: boolean) => void;
}

const LicenseActivation: React.FC<LicenseActivationProps> = ({ isProVersion, onActivationComplete }) => {
  const [licenseKey, setLicenseKey] = useState('');
  const [email, setEmail] = useState('');
  const [isActivating, setIsActivating] = useState(false);
  const [errorMessage, setErrorMessage] = useState('');
  const [successMessage, setSuccessMessage] = useState('');

  const validateEmail = (email: string): boolean => {
    return /\S+@\S+\.\S+/.test(email);
  };

  const validateLicenseKey = (key: string): boolean => {
    // PRO-XXXX-XXXX-XXXX format
    return /^PRO-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/.test(key);
  };

  const handleActivate = async (e: React.FormEvent): Promise<void> => {
    e.preventDefault();
    
    // Reset messages
    setErrorMessage('');
    setSuccessMessage('');
    
    // Basic validation
    if (!licenseKey) {
      setErrorMessage('Please enter your license key');
      return;
    }
    
    if (!email) {
      setErrorMessage('Please enter your email address');
      return;
    }
    
    if (!validateEmail(email)) {
      setErrorMessage('Please enter a valid email address');
      return;
    }
    
    if (!validateLicenseKey(licenseKey)) {
      setErrorMessage('License key should be in the format PRO-XXXX-XXXX-XXXX');
      return;
    }
    
    // Start activation process
    setIsActivating(true);
    
    try {
      // Fixed: changed to the correct command name 'activate_license'
      await invoke('activate_license', {
        licenseKey,
        email
      });
      
      setSuccessMessage('License activated successfully!');
      setLicenseKey('');
      setEmail('');
      
      // Notify parent component
      if (onActivationComplete) {
        onActivationComplete(true);
      }
      setIsActivating(false);
    } catch (error) {
      setErrorMessage(`Activation failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
      
      // Notify parent component
      if (onActivationComplete) {
        onActivationComplete(false);
      }
      setIsActivating(false);
    }
  };

  // If already on Pro version, show different content
  if (isProVersion) {
    return (
      <div className="bg-green-50 dark:bg-green-900 p-6 rounded-lg shadow-md">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-green-800 dark:text-green-200">Pro License Active</h2>
          <span className="px-3 py-1 bg-green-500 text-white text-sm font-medium rounded-full">
            ACTIVE
          </span>
        </div>
        <p className="text-green-700 dark:text-green-300 mb-4">
          Thank you for using Rustloader Pro! You have access to all premium features:
        </p>
        <ul className="list-disc list-inside text-green-700 dark:text-green-300 space-y-1 mb-4">
          <li>4K/8K video quality downloads</li>
          <li>High-fidelity audio formats (FLAC, 320kbps MP3)</li>
          <li>No daily download limits</li>
          <li>Multi-threaded downloads for maximum speed</li>
          <li>Priority updates and support</li>
        </ul>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-md">
      <h2 className="text-lg font-semibold text-gray-800 dark:text-white mb-4">
        Activate Pro License
      </h2>
      
      {errorMessage && (
        <div className="bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200 p-3 rounded-md mb-4 text-sm">
          {errorMessage}
        </div>
      )}
      
      {successMessage && (
        <div className="bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-200 p-3 rounded-md mb-4 text-sm">
          {successMessage}
        </div>
      )}
      
      <form onSubmit={handleActivate} className="space-y-4">
        <div className="space-y-2">
          <label htmlFor="license-key" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            License Key
          </label>
          <input
            type="text"
            id="license-key"
            value={licenseKey}
            onChange={(e) => setLicenseKey(e.target.value.toUpperCase())}
            placeholder="PRO-XXXX-XXXX-XXXX"
            className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            disabled={isActivating}
          />
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Enter your license key in the format PRO-XXXX-XXXX-XXXX
          </p>
        </div>
        
        <div className="space-y-2">
          <label htmlFor="email" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Email Address
          </label>
          <input
            type="email"
            id="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="your@email.com"
            className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            disabled={isActivating}
          />
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Enter the email address used to purchase your license
          </p>
        </div>
        
        <button
          type="submit"
          disabled={isActivating || !licenseKey || !email}
          className="w-full py-2 px-4 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md shadow-sm disabled:bg-blue-300 transition-colors"
        >
          {isActivating ? 'Activating...' : 'Activate License'}
        </button>
      </form>
      
      <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
        <p className="text-sm text-gray-600 dark:text-gray-400 text-center">
          Don't have a license? <a href="#" className="text-blue-600 dark:text-blue-400 hover:underline">Purchase Pro</a>
        </p>
      </div>
    </div>
  );
};

export default LicenseActivation;