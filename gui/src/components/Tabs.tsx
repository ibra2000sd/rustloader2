// src/components/Tabs.tsx
import React from 'react';
import { TabType } from '../App';

interface TabsProps {
  activeTab: TabType;
  onChange: (tab: TabType) => void;
}

const Tabs: React.FC<TabsProps> = ({ activeTab, onChange }) => {
  return (
    <div className="border-b border-gray-200 dark:border-gray-700">
      <nav className="flex space-x-8">
        <button
          onClick={() => onChange('download')}
          className={`py-2 px-1 font-medium text-sm border-b-2 ${
            activeTab === 'download'
              ? 'border-primary-500 text-primary-600 dark:text-primary-400'
              : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
          }`}
        >
          Download
        </button>
        <button
          onClick={() => onChange('license')}
          className={`py-2 px-1 font-medium text-sm border-b-2 ${
            activeTab === 'license'
              ? 'border-primary-500 text-primary-600 dark:text-primary-400'
              : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
          }`}
        >
          License
        </button>
      </nav>
    </div>
  );
};

export default Tabs;