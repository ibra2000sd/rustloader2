import React, { useState } from 'react';
import { 
  BookOpen, 
  HelpCircle, 
  Video, 
  FileText, 
  Clipboard, 
  Zap, 
  Search, 
  AlertTriangle, 
  Terminal, 
  PanelRight, 
  ChevronDown, 
  ChevronUp, 
  ThumbsUp, 
  ThumbsDown, 
  ExternalLink 
} from 'lucide-react';

// Helper component for expandable sections
const ExpandableSection = ({ 
  title, 
  children, 
  icon, 
  defaultOpen = false 
}: { 
  title: string; 
  children: React.ReactNode; 
  icon: React.ReactNode; 
  defaultOpen?: boolean 
}) => {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  
  return (
    <div className="border dark:border-gray-700 rounded-lg overflow-hidden mb-4">
      <button
        className={`w-full p-4 flex items-center justify-between bg-gray-50 dark:bg-gray-800 text-left transition ${
          isOpen ? 'border-b dark:border-gray-700' : ''
        }`}
        onClick={() => setIsOpen(!isOpen)}
      >
        <div className="flex items-center">
          {icon}
          <span className="ml-2 font-medium">{title}</span>
        </div>
        {isOpen ? <ChevronUp size={20} /> : <ChevronDown size={20} />}
      </button>
      
      {isOpen && (
        <div className="p-4 bg-white dark:bg-gray-900">
          {children}
        </div>
      )}
    </div>
  );
};

// FAQ Item component
const FAQItem = ({ 
  question, 
  answer 
}: { 
  question: string; 
  answer: React.ReactNode 
}) => {
  const [isOpen, setIsOpen] = useState(false);
  
  return (
    <div className="border-b dark:border-gray-700 last:border-b-0 py-3">
      <button
        className="w-full flex items-center justify-between text-left"
        onClick={() => setIsOpen(!isOpen)}
      >
        <span className="font-medium text-gray-900 dark:text-gray-100">{question}</span>
        {isOpen ? <ChevronUp size={18} /> : <ChevronDown size={18} />}
      </button>
      
      {isOpen && (
        <div className="mt-2 text-gray-600 dark:text-gray-300 text-sm">
          {answer}
        </div>
      )}
    </div>
  );
};

// Sample URL component
const SampleURL = ({ 
  url, 
  description 
}: { 
  url: string; 
  description: string 
}) => {
  const copyToClipboard = () => {
    navigator.clipboard.writeText(url);
  };
  
  return (
    <div className="flex items-center justify-between p-2 bg-gray-50 dark:bg-gray-800 rounded mb-2 text-sm">
      <div>
        <div className="font-mono text-gray-800 dark:text-gray-200 truncate max-w-xs">{url}</div>
        <div className="text-xs text-gray-500 dark:text-gray-400">{description}</div>
      </div>
      <button
        onClick={copyToClipboard}
        className="ml-2 p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
        title="Copy to clipboard"
      >
        <Clipboard size={16} />
      </button>
    </div>
  );
};

// Interactive example card
const ExampleCard = ({ 
  title, 
  screenshot, 
  steps 
}: { 
  title: string; 
  screenshot: string; 
  steps: string[] 
}) => {
  return (
    <div className="border dark:border-gray-700 rounded-lg overflow-hidden">
      <div className="p-3 bg-gray-50 dark:bg-gray-800 border-b dark:border-gray-700">
        <h3 className="font-medium">{title}</h3>
      </div>
      
      <div className="p-3">
        <div className="mb-3 border dark:border-gray-700 rounded overflow-hidden">
          <img src={screenshot} alt={title} className="w-full h-auto" />
        </div>
        
        <h4 className="font-medium mb-2 text-sm">Steps:</h4>
        <ol className="list-decimal list-inside text-sm space-y-1 text-gray-700 dark:text-gray-300">
          {steps.map((step, index) => (
            <li key={index}>{step}</li>
          ))}
        </ol>
      </div>
    </div>
  );
};

// Main Help Center component
const HelpCenter: React.FC = () => {
  const [searchTerm, setSearchTerm] = useState('');
  const [feedbackSubmitted, setFeedbackSubmitted] = useState(false);
  
  // Filter content based on search term
  const filterContent = (text: string) => {
    if (!searchTerm) return true;
    return text.toLowerCase().includes(searchTerm.toLowerCase());
  };

  // Handle feedback submission
  const submitFeedback = (helpful: boolean) => {
    // In a real app, this would send feedback to the server
    console.log(`User found help ${helpful ? 'helpful' : 'not helpful'}`);
    setFeedbackSubmitted(true);
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden">
      <div className="p-4 bg-blue-600 text-white flex items-center">
        <HelpCircle size={24} className="mr-2" />
        <h2 className="text-xl font-semibold">Help & Support Center</h2>
      </div>
      
      {/* Search bar */}
      <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
        <div className="relative">
          <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <Search size={18} className="text-gray-400" />
          </div>
          <input
            type="text"
            placeholder="Search help topics..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="block w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500"
          />
        </div>
      </div>
      
      <div className="p-4 overflow-auto max-h-[70vh]">
        {/* Quick Start Section */}
        {filterContent('quick start guide getting started basics') && (
          <ExpandableSection 
            title="Quick Start Guide" 
            icon={<Zap size={18} className="text-yellow-500" />}
            defaultOpen={true}
          >
            <div className="space-y-4">
              <p className="text-gray-700 dark:text-gray-300">
                Rustloader makes it easy to download videos and audio from popular websites. Follow these simple steps:
              </p>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <ExampleCard
                  title="Downloading a YouTube Video"
                  screenshot="/examples/youtube-download.png"
                  steps={[
                    "Copy the URL from YouTube",
                    "Paste it in the URL field",
                    "Select your preferred format and quality",
                    "Click Download"
                  ]}
                />
                
                <ExampleCard
                  title="Extracting Audio from a Video"
                  screenshot="/examples/audio-extract.png"
                  steps={[
                    "Copy the video URL",
                    "Paste it in the URL field",
                    "Change format to MP3 or other audio format",
                    "Adjust audio quality if needed",
                    "Click Download"
                  ]}
                />
              </div>
              
              <div className="mt-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
                <h4 className="font-medium text-blue-800 dark:text-blue-300 mb-2">Pro Tip</h4>
                <p className="text-blue-700 dark:text-blue-200 text-sm">
                  You can download only a specific portion of a video by using the trim feature. Just set the start and end times in the advanced options.
                </p>
              </div>
            </div>
          </ExpandableSection>
        )}
        
        {/* Supported Sites Section */}
        {filterContent('supported sites platforms youtube vimeo dailymotion') && (
          <ExpandableSection 
            title="Supported Sites" 
            icon={<Video size={18} className="text-red-500" />}
          >
            <div>
              <p className="mb-4 text-gray-700 dark:text-gray-300">
                Rustloader supports downloading from hundreds of websites, including:
              </p>
              
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2 mb-4">
                {['YouTube', 'Vimeo', 'Dailymotion', 'Facebook', 'Twitter', 'Instagram', 'TikTok', 'SoundCloud', 'Twitch', 'Bandcamp', 'Reddit', 'LinkedIn', 'Bilibili', 'Imgur', 'TED', 'Udemy'].map((site) => (
                  <div 
                    key={site} 
                    className="p-2 bg-gray-50 dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700 text-sm text-center"
                  >
                    {site}
                  </div>
                ))}
              </div>
              
              <h4 className="font-medium mb-2">Try with these examples:</h4>
              <div className="space-y-2">
                <SampleURL
                  url="https://www.youtube.com/watch?v=dQw4w9WgXcQ"
                  description="YouTube video"
                />
                <SampleURL
                  url="https://vimeo.com/148751763"
                  description="Vimeo video"
                />
                <SampleURL
                  url="https://soundcloud.com/artist/track"
                  description="SoundCloud audio"
                />
              </div>
              
              <div className="mt-3 text-sm text-gray-600 dark:text-gray-400">
                <a href="#" className="text-blue-600 dark:text-blue-400 hover:underline inline-flex items-center">
                  <span>View full list of supported sites</span>
                  <ExternalLink size={14} className="ml-1" />
                </a>
              </div>
            </div>
          </ExpandableSection>
        )}
        
        {/* Troubleshooting Section */}
        {filterContent('troubleshooting problems issues error fix solution') && (
          <ExpandableSection 
            title="Troubleshooting" 
            icon={<AlertTriangle size={18} className="text-orange-500" />}
          >
            <div>
              <p className="mb-4 text-gray-700 dark:text-gray-300">
                Encountering issues? Here are solutions to common problems:
              </p>
              
              <div className="space-y-4">
                <FAQItem
                  question="Download fails immediately"
                  answer={
                    <div>
                      <p className="mb-2">This usually happens due to one of these reasons:</p>
                      <ul className="list-disc list-inside pl-2 space-y-1">
                        <li>Invalid or unsupported URL</li>
                        <li>Video is unavailable or has been removed</li>
                        <li>Network connection issues</li>
                        <li>Missing system dependencies</li>
                      </ul>
                      <div className="mt-2 p-2 bg-gray-50 dark:bg-gray-800 rounded text-sm">
                        <strong>Fix:</strong> First, verify your internet connection. If that's not the issue, try checking if the video is still available by opening it directly in your browser.
                      </div>
                    </div>
                  }
                />
                
                <FAQItem
                  question="Certain quality options are unavailable"
                  answer={
                    <div>
                      <p className="mb-2">Not all quality options are available for every video. The available options depend on:</p>
                      <ul className="list-disc list-inside pl-2 space-y-1">
                        <li>What the platform offers for that specific content</li>
                        <li>Your license type (some high resolutions require Rustloader Pro)</li>
                      </ul>
                      <div className="mt-2 p-2 bg-gray-50 dark:bg-gray-800 rounded text-sm">
                        <strong>Fix:</strong> Try selecting a different quality option, or <a href="#" className="text-blue-600 dark:text-blue-400 hover:underline">upgrade to Pro</a> for more options.
                      </div>
                    </div>
                  }
                />
                
                <FAQItem
                  question="Downloads are very slow"
                  answer={
                    <div>
                      <p className="mb-2">Slow downloads can be caused by:</p>
                      <ul className="list-disc list-inside pl-2 space-y-1">
                        <li>Your internet connection speed</li>
                        <li>Server limitations from the source website</li>
                        <li>Too many concurrent downloads</li>
                      </ul>
                      <div className="mt-2 p-2 bg-gray-50 dark:bg-gray-800 rounded text-sm">
                        <strong>Fix:</strong> Try limiting concurrent downloads in Settings. Also, some platforms throttle download speeds to prevent excessive bandwidth usage.
                      </div>
                    </div>
                  }
                />
                
                <FAQItem
                  question="Audio and video are out of sync"
                  answer={
                    <div>
                      <p>This can happen when downloading high-resolution videos with separate audio tracks.</p>
                      <div className="mt-2 p-2 bg-gray-50 dark:bg-gray-800 rounded text-sm">
                        <strong>Fix:</strong> Try downloading in a different format, or enable the "Use alternative merger" option in advanced settings.
                      </div>
                    </div>
                  }
                />
              </div>
              
              <div className="mt-6 p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
                <h4 className="font-medium text-yellow-800 dark:text-yellow-300 mb-2">Still having issues?</h4>
                <p className="text-yellow-700 dark:text-yellow-200 text-sm mb-2">
                  Try running the diagnostics tool to identify and fix common problems automatically.
                </p>
                <button className="px-3 py-1.5 bg-yellow-100 dark:bg-yellow-800 text-yellow-800 dark:text-yellow-200 rounded text-sm font-medium hover:bg-yellow-200 dark:hover:bg-yellow-700 flex items-center">
                  <Terminal size={14} className="mr-1" />
                  Run Diagnostics
                </button>
              </div>
            </div>
          </ExpandableSection>
        )}
        
        {/* Advanced Features Section */}
        {filterContent('advanced features pro capabilities') && (
          <ExpandableSection 
            title="Advanced Features" 
            icon={<PanelRight size={18} className="text-purple-500" />}
          >
            <div>
              <p className="mb-4 text-gray-700 dark:text-gray-300">
                Rustloader includes several advanced features for power users:
              </p>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div className="p-3 border dark:border-gray-700 rounded-lg">
                  <h4 className="font-medium mb-2">Command Line Interface</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">
                    Use Rustloader from your terminal for automation and scripting.
                  </p>
                  <div className="bg-gray-800 text-gray-200 p-2 rounded text-xs font-mono overflow-x-auto">
                    $ rustloader download https://example.com/video --format mp4 --quality 720
                  </div>
                </div>
                
                <div className="p-3 border dark:border-gray-700 rounded-lg">
                  <h4 className="font-medium mb-2">Batch Downloads</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">
                    Download multiple videos at once from a text file.
                  </p>
                  <div className="bg-gray-800 text-gray-200 p-2 rounded text-xs font-mono overflow-x-auto">
                    $ rustloader batch downloads.txt --output ~/Videos
                  </div>
                </div>
              </div>
              
              <h4 className="font-medium mb-2">Pro Features</h4>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 mb-4">
                {[
                  { name: '4K Video Downloads', pro: true },
                  { name: 'Unlimited Concurrent Downloads', pro: true },
                  { name: 'Priority Download Queue', pro: true },
                  { name: 'High-Quality Audio (FLAC)', pro: true },
                  { name: 'Custom Output Templates', pro: true },
                  { name: 'Batch Processing', pro: false },
                  { name: 'Subtitle Downloads', pro: false },
                  { name: 'Video Trimming', pro: false },
                ].map((feature) => (
                  <div 
                    key={feature.name} 
                    className={`p-2 rounded text-sm flex items-center ${
                      feature.pro
                        ? 'bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800'
                        : 'bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700'
                    }`}
                  >
                    <span>{feature.name}</span>
                    {feature.pro && (
                      <span className="ml-auto px-1.5 py-0.5 text-xs bg-yellow-500 text-white rounded-sm">PRO</span>
                    )}
                  </div>
                ))}
              </div>
              
              <div className="text-center">
                <a 
                  href="#"
                  className="inline-block px-4 py-2 bg-gradient-to-r from-yellow-400 to-yellow-600 text-white rounded-md font-medium hover:from-yellow-500 hover:to-yellow-700 transition-all"
                >
                  Upgrade to Rustloader Pro
                </a>
              </div>
            </div>
          </ExpandableSection>
        )}
        
        {/* Documentation Section */}
        {filterContent('documentation docs manual guide reference') && (
          <ExpandableSection 
            title="Documentation" 
            icon={<BookOpen size={18} className="text-green-500" />}
          >
            <div>
              <p className="mb-4 text-gray-700 dark:text-gray-300">
                Access comprehensive documentation for all Rustloader features:
              </p>
              
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <a href="#" className="block p-4 border dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition">
                  <FileText size={24} className="text-blue-500 mb-2" />
                  <h4 className="font-medium mb-1">User Guide</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Complete guide to all features and options
                  </p>
                </a>
                
                <a href="#" className="block p-4 border dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition">
                  <Terminal size={24} className="text-gray-500 mb-2" />
                  <h4 className="font-medium mb-1">CLI Reference</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Command-line options and examples
                  </p>
                </a>
                
                <a href="#" className="block p-4 border dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition">
                  <Video size={24} className="text-red-500 mb-2" />
                  <h4 className="font-medium mb-1">Video Tutorials</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Step-by-step video instructions
                  </p>
                </a>
              </div>
              
              <div className="mt-4 text-sm text-gray-600 dark:text-gray-400">
                Looking for something specific? Check our <a href="#" className="text-blue-600 dark:text-blue-400 hover:underline">API documentation</a> or <a href="#" className="text-blue-600 dark:text-blue-400 hover:underline">Frequently Asked Questions</a>.
              </div>
            </div>
          </ExpandableSection>
        )}
      </div>
      
      {/* Feedback section */}
      <div className="p-4 bg-gray-50 dark:bg-gray-700 border-t border-gray-200 dark:border-gray-600">
        {!feedbackSubmitted ? (
          <div className="text-center">
            <p className="text-sm text-gray-600 dark:text-gray-300 mb-2">Was this help section useful?</p>
            <div className="flex justify-center space-x-4">
              <button
                onClick={() => submitFeedback(true)}
                className="px-4 py-2 flex items-center text-sm text-green-600 dark:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20 rounded"
              >
                <ThumbsUp size={16} className="mr-1" />
                Yes, it helped
              </button>
              <button
                onClick={() => submitFeedback(false)}
                className="px-4 py-2 flex items-center text-sm text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
              >
                <ThumbsDown size={16} className="mr-1" />
                No, I need more help
              </button>
            </div>
          </div>
        ) : (
          <div className="text-center text-sm text-gray-600 dark:text-gray-300">
            Thanks for your feedback! <a href="#" className="text-blue-600 dark:text-blue-400 hover:underline">Contact support</a> for additional help.
          </div>
        )}
      </div>
    </div>
  );
};

export default HelpCenter;