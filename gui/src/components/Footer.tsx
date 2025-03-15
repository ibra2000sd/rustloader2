// src/components/Footer.tsx
import React from 'react';

const Footer: React.FC = () => {
  return (
    <footer className="py-3 px-6 bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 text-xs text-gray-500 dark:text-gray-400">
      <div className="flex justify-between items-center">
        <div>
          Rustloader v1.0.0
        </div>
        <div>
          © 2025 Rustloader
        </div>
      </div>
    </footer>
  );
};

export default Footer;