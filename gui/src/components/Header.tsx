// src/components/Header.tsx
import React from 'react';
import { LicenseStatus } from '../App.jsx';

interface HeaderProps {
  licenseStatus: LicenseStatus;
}

const Header: React.FC<HeaderProps> = ({ licenseStatus }) => {
  return (
    <header className="bg-white dark:bg-gray-800 shadow-sm py-4 px-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <div className="text-primary-600 dark:text-primary-400">
            <svg 
              xmlns="http://www.w3.org/2000/svg" 
              viewBox="0 0 24 24"
              fill="currentColor"
              className="w-8 h-8"
            >
              <path d="M4.5 4.5a3 3 0 00-3 3v9a3 3 0 003 3h8.25a3 3 0 003-3v-9a3 3 0 00-3-3H4.5zM19.94 18.75l-2.69-2.69V7.94l2.69-2.69c.944-.945 2.56-.276 2.56 1.06v11.38c0 1.336-1.616 2.005-2.56 1.06z" />
            </svg>
          </div>
          <div>
            <h1 className="text-xl font-bold text-gray-900 dark:text-white">Rustloader</h1>
            <p className="text-xs text-gray-500 dark:text-gray-400">Advanced Video Downloader</p>
          </div>
        </div>
        
        <div>
          <span 
            className={`inline-flex items-center rounded-full px-3 py-0.5 text-sm font-medium ${
              licenseStatus === 'checking' 
                ? 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300' 
                : licenseStatus === 'pro'
                ? 'bg-pro-500 text-white' 
                : 'bg-free-500 text-white'
            }`}
          >
            {licenseStatus === 'checking' 
              ? 'Checking...' 
              : licenseStatus === 'pro' 
              ? 'PRO VERSION' 
              : 'FREE VERSION'}
          </span>
        </div>
      </div>
    </header>
  );
};

export default Header;