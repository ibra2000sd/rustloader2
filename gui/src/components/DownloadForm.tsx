import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import Alert from './Alert';

interface DownloadFormProps {
  isPro: boolean;
  onDownloadStart: (params: {
    url: string;
    quality: string;
    format: string;
    startTime?: string;
    endTime?: string;
    usePlaylist: boolean;
    downloadSubtitles: boolean;
    outputDir?: string;
  }) => void;
  disabled?: boolean;
}

interface VideoInfo {
  title?: string;
  uploader?: string;
  duration?: number;
  views?: number;
  likes?: number;
  uploadDate?: string;
}

const isValidUrl = (string: string): boolean => {
  try {
    const url = new URL(string);
    return url.protocol === 'http:' || url.protocol === 'https:';
  } catch (_) {
    return false;
  }
};

const isVideoUrl = (string: string): boolean => {
  try {
    const url = new URL(string);
    const videoHosts = [
      'youtube.com', 'youtu.be', 'vimeo.com', 'dailymotion.com',
      'twitch.tv', 'tiktok.com', 'facebook.com', 'instagram.com'
    ];
    return videoHosts.some((host) => url.hostname.includes(host));
  } catch (_) {
    return false;
  }
};

const DownloadForm: React.FC<DownloadFormProps> = ({
  isPro,
  onDownloadStart,
  disabled = false
}) => {
  const [url, setUrl] = useState('');
  const [quality, setQuality] = useState(isPro ? '1080' : '720');
  const [format, setFormat] = useState('mp4');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [usePlaylist, setUsePlaylist] = useState(false);
  const [downloadSubtitles, setDownloadSubtitles] = useState(false);
  const [outputDir, setOutputDir] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);
  const [availablePaths, setAvailablePaths] = useState<string[]>([]);
  const [isUrlValid, setIsUrlValid] = useState(false);

  useEffect(() => {
    if (!isPro && (quality === '1080' || quality === '2160')) {
      setQuality('720');
    }
  }, [isPro, quality]);

  useEffect(() => {
    const loadPaths = async () => {
      try {
        let paths: string[] = [];
        try {
          paths = await invoke<string[]>('list_download_paths');
        } catch (err) {
          console.warn('Could not retrieve download paths from backend, using defaults');
          const homePath = process.env.HOME || '';
          paths = [
            `${homePath}/Downloads/rustloader/videos`,
            `${homePath}/Downloads/rustloader/audio`
          ].filter(Boolean);
        }
        setAvailablePaths(paths);
        if (paths.length > 0 && !outputDir) {
          setOutputDir(paths[0]);
        }
      } catch (error) {
        console.error('Failed to load available paths:', error);
      }
    };
    loadPaths();
  }, [outputDir]);

  useEffect(() => {
    setError('');
  }, [url]);

  const handleUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newUrl = e.target.value;
    setUrl(newUrl);
    const valid = isValidUrl(newUrl) && isVideoUrl(newUrl);
    setIsUrlValid(valid);
    if (error) setError('');
  };

  const formatDuration = (seconds?: number): string => {
    if (!seconds) return '00:00';
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  };

  const fetchVideoInfo = async (): Promise<void> => {
    if (!url || !isValidUrl(url)) {
      setError('Please enter a valid URL');
      return;
    }

    setIsLoading(true);
    setError('');
    setVideoInfo(null);

    try {
      const videoData = await invoke<VideoInfo>('get_video_info', { url });
      setVideoInfo(videoData);
    } catch (err) {
      console.error('Error fetching video info:', err);
      setError(
        `Failed to fetch video info: ${
          err instanceof Error ? err.message : 'Unknown error'
        }`
      );
    } finally {
      setIsLoading(false);
    }
  };

  const selectOutputDirectory = async (): Promise<void> => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Download Directory'
      });

      if (selected && typeof selected === 'string') {
        setOutputDir(selected);
      }
    } catch (err) {
      setError(
        `Failed to select directory: ${
          err instanceof Error ? err.message : 'Unknown error'
        }`
      );
    }
  };

  const validateTimeFormat = (value: string): boolean => {
    if (!value) return true;
    const timeRegex = /^([0-9][0-9]):([0-5][0-9]):([0-5][0-9])$/;
    return timeRegex.test(value);
  };

  const handleSubmit = (e: React.FormEvent): void => {
    e.preventDefault();

    if (!url) {
      setError('Please enter a URL');
      return;
    }

    if (startTime && !validateTimeFormat(startTime)) {
      setError('Invalid start time format. Use HH:MM:SS');
      return;
    }

    if (endTime && !validateTimeFormat(endTime)) {
      setError('Invalid end time format. Use HH:MM:SS');
      return;
    }

    onDownloadStart({
      url,
      quality,
      format,
      startTime: startTime || undefined,
      endTime: endTime || undefined,
      usePlaylist,
      downloadSubtitles,
      outputDir: outputDir || undefined
    });
  };

  return (
    <div>
      {error && <Alert type="error" message={error} onDismiss={() => setError('')} />}
      <form onSubmit={handleSubmit}>
        <input
          type="text"
          value={url}
          onChange={handleUrlChange}
          onBlur={() => url && isUrlValid && fetchVideoInfo()}
          placeholder="Enter video URL"
          disabled={isLoading || disabled}
          required
        />
        <button
          type="button"
          onClick={fetchVideoInfo}
          disabled={isLoading || disabled || !url || !isUrlValid}
        >
          Fetch Info
        </button>
        {videoInfo && (
          <div>
            <h3>{videoInfo.title || 'Unknown Title'}</h3>
            {videoInfo.duration && <span>Duration: {formatDuration(videoInfo.duration)}</span>}
            {videoInfo.uploader && <span>By: {videoInfo.uploader}</span>}
            {videoInfo.views && <span>Views: {videoInfo.views.toLocaleString()}</span>}
          </div>
        )}
        <button
          type="submit"
          disabled={isLoading || disabled || !url || !isUrlValid}
        >
          Download
        </button>
      </form>
    </div>
  );
};

export default DownloadForm;
