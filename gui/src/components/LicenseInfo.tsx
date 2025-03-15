// src/components/LicenseInfo.tsx
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import Alert from './Alert';

interface LicenseInfoProps {
  isProVersion: boolean;
  onActivationComplete: (success: boolean) => void;
}

const LicenseInfo: React.FC<LicenseInfoProps> = ({ isProVersion, onActivationComplete }) => {
  const [licenseKey, setLicenseKey] = useState('');
  const [email, setEmail] = useState('');
  const [isActivating, setIsActivating] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  // Validation functions
  const validateEmail = (email: string): boolean => {
    return /\S+@\S+\.\S+/.test(email);
  };

  const validateLicenseKey = (key: string): boolean => {
    // PRO-XXXX-XXXX-XXXX format
    return /^PRO-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/.test(key);
  };

  // Handle license activation
  const handleActivate = async (e: React.FormEvent) => {
    e.preventDefault();
    
    // Reset messages
    setError('');
    setSuccess('');
    
    // Basic validation
    if (!licenseKey) {
      setError('Please enter your license key');
      return;
    }
    
    if (!email) {
      setError('Please enter your email address');
      return;
    }
    
    if (!validateEmail(email)) {
      setError('Please enter a valid email address');
      return;
    }
    
    if (!validateLicenseKey(licenseKey)) {
      setError('License key should be in the format PRO-XXXX-XXXX-XXXX');
      return;
    }
    
    // Start activation process
    setIsActivating(true);
    
    try {
      await invoke('activate_license', {
        licenseKey,
        email
      });
      
      setSuccess('License activated successfully!');
      setLicenseKey('');
      setEmail('');
      
      // Notify parent component
      onActivationComplete(true);
    } catch (err) {
      setError(`Activation failed: ${err instanceof Error ? err.message : String(err)}`);
      onActivationComplete(false);
    } finally {
      setIsActivating(false);
    }
  };

  // If already on Pro version, show different content
  if (isProVersion) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-5">
        <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-5">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-green-800 dark:text-green-200">Pro License Active</h2>
            <span className="px-3 py-1 bg-green-500 text-white text-xs font-medium rounded-full">
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
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-5">
      <h2 className="text-lg font-semibold text-gray-800 dark:text-white mb-4">
        Activate Pro License
      </h2>
      
      {error && (
        <Alert type="error" message={error} onDismiss={() => setError('')} />
      )}
      
      {success && (
        <Alert type="success" message={success} onDismiss={() => setSuccess('')} />
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
            className="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white disabled:opacity-70"
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
            className="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white disabled:opacity-70"
            disabled={isActivating}
          />
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Enter the email address used to purchase your license
          </p>
        </div>
        
        <button
          type="submit"
          disabled={isActivating || !licenseKey || !email}
          className="w-full py-2.5 px-4 bg-primary-600 hover:bg-primary-700 text-white font-medium rounded-md shadow-sm transition-colors disabled:bg-primary-400 disabled:cursor-not-allowed"
        >
          {isActivating ? 'Activating...' : 'Activate License'}
        </button>
      </form>
      
      <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
        <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
          <h3 className="text-base font-medium text-gray-800 dark:text-gray-200 mb-2">
            Pro Version Features
          </h3>
          <ul className="list-disc list-inside text-sm text-gray-600 dark:text-gray-400 space-y-1">
            <li>Download videos in 4K and 8K quality</li>
            <li>Extract audio in high-fidelity FLAC and 320Kbps MP3</li>
            <li>No daily download limits</li>
            <li>Multi-threaded downloads for maximum speed</li>
            <li>Priority updates and technical support</li>
          </ul>
        </div>

        <p className="mt-4 text-center text-sm text-gray-600 dark:text-gray-400">
          Don't have a license yet? <a href="https://rustloader.com/pro" target="_blank" rel="noopener noreferrer" className="text-primary-600 dark:text-primary-400 hover:underline">Purchase Pro</a>
        </p>
      </div>
    </div>
  );
};

export default LicenseInfo;