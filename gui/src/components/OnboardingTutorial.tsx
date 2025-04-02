import React, { useState, useEffect } from 'react';
import { Store } from '@tauri-apps/plugin-store';
import { Download, Video, Music, Settings, Info, Check, ChevronRight, ChevronLeft, X } from 'lucide-react';

interface OnboardingTutorialProps {
  onComplete: () => void;
  onSkip: () => void;
}

interface Step {
  id: string;
  title: string;
  description: string;
  icon: React.ReactNode;
  image?: string;
}

const OnboardingTutorial: React.FC<OnboardingTutorialProps> = ({ onComplete, onSkip }) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [doNotShowAgain, setDoNotShowAgain] = useState(false);

  // Define our onboarding steps
  const steps: Step[] = [
    {
      id: 'welcome',
      title: 'Welcome to Rustloader',
      description: 'Your fast, secure and powerful video downloader. Let\'s take a quick tour to get you started.',
      icon: <Download size={32} className="text-blue-500" />,
      image: '/onboarding/welcome.png'
    },
    {
      id: 'video-download',
      title: 'Download Videos',
      description: 'Paste any URL from YouTube, Vimeo, or other supported sites. Choose quality and format, then click Download.',
      icon: <Video size={32} className="text-purple-500" />,
      image: '/onboarding/video-download.png'
    },
    {
      id: 'audio-download',
      title: 'Extract Audio',
      description: 'Want just the audio? Select MP3 or other audio formats for music, podcasts, and more.',
      icon: <Music size={32} className="text-green-500" />,
      image: '/onboarding/audio-download.png'
    },
    {
      id: 'settings',
      title: 'Customize Settings',
      description: 'Configure default download location, preferred formats, and other options in the Settings tab.',
      icon: <Settings size={32} className="text-yellow-500" />,
      image: '/onboarding/settings.png'
    },
    {
      id: 'help',
      title: 'Need Help?',
      description: 'Check the Help section for troubleshooting, supported sites, and detailed usage guides.',
      icon: <Info size={32} className="text-red-500" />,
      image: '/onboarding/help.png'
    }
  ];

  // Save user preference about not showing onboarding again
  const handleComplete = async () => {
    if (doNotShowAgain) {
      try {
        const store = new Store('preferences.dat');
        await store.set('showOnboarding', false);
        await store.save();
      } catch (error) {
        console.error('Failed to save onboarding preference:', error);
      }
    }
    onComplete();
  };

  // Navigate between steps
  const nextStep = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      handleComplete();
    }
  };

  const prevStep = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const currentStepData = steps[currentStep];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-75">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-2xl w-full mx-4 overflow-hidden">
        {/* Header */}
        <div className="flex justify-between items-center p-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-xl font-semibold text-gray-800 dark:text-gray-100">
            {currentStepData.title}
          </h2>
          <button
            onClick={onSkip}
            className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6">
          <div className="flex flex-col md:flex-row items-center md:items-start gap-6">
            <div className="flex-shrink-0 p-3 bg-gray-100 dark:bg-gray-700 rounded-full">
              {currentStepData.icon}
            </div>
            <div className="flex-grow text-center md:text-left">
              <p className="text-gray-600 dark:text-gray-300 mb-4">
                {currentStepData.description}
              </p>
              {currentStepData.image && (
                <div className="mt-4 rounded-lg overflow-hidden border border-gray-200 dark:border-gray-700">
                  <img 
                    src={currentStepData.image} 
                    alt={currentStepData.title} 
                    className="w-full h-auto object-cover" 
                  />
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Progress dots */}
        <div className="flex justify-center py-3">
          {steps.map((_, index) => (
            <div
              key={index}
              className={`w-2 h-2 mx-1 rounded-full ${
                index === currentStep
                  ? 'bg-blue-500'
                  : 'bg-gray-300 dark:bg-gray-600'
              }`}
            />
          ))}
        </div>

        {/* Footer */}
        <div className="flex justify-between items-center p-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750">
          <div className="flex items-center">
            <input
              type="checkbox"
              id="doNotShowAgain"
              checked={doNotShowAgain}
              onChange={(e) => setDoNotShowAgain(e.target.checked)}
              className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label
              htmlFor="doNotShowAgain"
              className="ml-2 text-sm text-gray-600 dark:text-gray-400"
            >
              Don't show this again
            </label>
          </div>
          <div className="flex space-x-2">
            {currentStep > 0 && (
              <button
                onClick={prevStep}
                className="px-4 py-2 flex items-center text-sm text-gray-700 dark:text-gray-300 bg-gray-200 dark:bg-gray-600 rounded hover:bg-gray-300 dark:hover:bg-gray-500"
              >
                <ChevronLeft size={16} className="mr-1" /> Previous
              </button>
            )}
            <button
              onClick={nextStep}
              className="px-4 py-2 flex items-center text-sm text-white bg-blue-600 rounded hover:bg-blue-700"
            >
              {currentStep === steps.length - 1 ? (
                <>
                  <Check size={16} className="mr-1" /> Get Started
                </>
              ) : (
                <>
                  Next <ChevronRight size={16} className="ml-1" />
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default OnboardingTutorial;