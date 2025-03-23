import React, { useState, useEffect } from "react";
import {
  PlusCircle,
  Play,
  Pause,
  Download,
  CheckCircle,
  XCircle,
  Clock,
} from "lucide-react";

// Standalone component that doesn't rely on external imports
const DownloadManager = () => {
  const [downloads, setDownloads] = useState([]);
  const [nextId, setNextId] = useState(1);

  // Simulated download progress update
  useEffect(() => {
    const interval = setInterval(() => {
      setDownloads((current) =>
        current.map((download) => {
          if (download.status !== "downloading" || download.progress >= 100) {
            return download;
          }

          // Calculate new progress
          const progressIncrement = Math.random() * 5;
          const newProgress = Math.min(
            download.progress + progressIncrement,
            100,
          );

          // Calculate new speed (fluctuating slightly)
          const baseSpeed = download.speed || 1024 * 1024; // 1MB/s
          const speedVariation = baseSpeed * 0.2; // 20% variation
          const newSpeed =
            baseSpeed + Math.random() * speedVariation - speedVariation / 2;

          // Calculate remaining size and time
          const remainingSize = download.fileSize * (1 - newProgress / 100);
          const timeRemaining = newSpeed > 0 ? remainingSize / newSpeed : 0;

          // Update downloaded size
          const downloadedSize = download.fileSize * (newProgress / 100);

          // Check if download completed
          const newStatus = newProgress >= 100 ? "complete" : download.status;

          return {
            ...download,
            progress: newProgress,
            downloadedSize,
            speed: newSpeed,
            timeRemaining,
            status: newStatus,
          };
        }),
      );
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  const handleAddDownload = () => {
    const videoQualities = ["480p", "720p", "1080p", "4K"];
    const videoFormats = ["mp4", "webm", "mkv"];
    const audioFormats = ["mp3", "aac", "flac"];

    const isAudio = Math.random() > 0.7;
    const format = isAudio
      ? audioFormats[Math.floor(Math.random() * audioFormats.length)]
      : videoFormats[Math.floor(Math.random() * videoFormats.length)];

    const quality = isAudio
      ? Math.random() > 0.5
        ? "128kbps"
        : "320kbps"
      : videoQualities[Math.floor(Math.random() * videoQualities.length)];

    // Random file size between 50MB and 2GB
    const fileSize = isAudio
      ? Math.random() * 50 * 1024 * 1024 + 10 * 1024 * 1024
      : Math.random() * 2 * 1024 * 1024 * 1024 + 50 * 1024 * 1024;

    const newDownload = {
      id: nextId,
      title: isAudio
        ? `Music Track ${nextId} - Artist Name`
        : `Video Title ${nextId} - Channel Name`,
      url: "https://example.com/video",
      progress: 0,
      fileSize,
      downloadedSize: 0,
      speed: Math.random() * 5 * 1024 * 1024 + 500 * 1024, // 500KB to 5MB/s
      timeRemaining: 0,
      format,
      quality,
      status: "downloading",
      isPaused: false,
    };

    setDownloads((current) => [...current, newDownload]);
    setNextId(nextId + 1);
  };

  const handlePauseResume = (id) => {
    setDownloads((current) =>
      current.map((download) => {
        if (download.id === id) {
          const isPaused = !download.isPaused;
          return {
            ...download,
            isPaused,
            status: isPaused ? "paused" : "downloading",
          };
        }
        return download;
      }),
    );
  };

  const handleCancel = (id) => {
    setDownloads((current) => current.filter((download) => download.id !== id));
  };

  // Utility functions for formatting
  const formatBytes = (bytes, decimals = 2) => {
    if (bytes === 0) return "0 Bytes";

    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ["Bytes", "KB", "MB", "GB", "TB"];

    const i = Math.floor(Math.log(bytes) / Math.log(k));

    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i];
  };

  const formatTime = (seconds) => {
    if (!seconds) return "--:--";

    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);

    return `${String(minutes).padStart(2, "0")}:${String(remainingSeconds).padStart(2, "0")}`;
  };

  // Embedded download card component
  const DownloadInfoCard = ({
    downloadInfo,
    isPaused,
    onPauseResume,
    onCancel,
  }) => {
    const {
      title = "Downloading...",
      url = "",
      progress = 0,
      fileSize = 0,
      downloadedSize = 0,
      speed = 0,
      timeRemaining = 0,
      format = "mp4",
      quality = "720p",
      status = "downloading", // 'downloading', 'complete', 'error', 'paused'
    } = downloadInfo;

    const [isPlaying, setIsPlaying] = useState(false);

    // Play animation when download completes
    useEffect(() => {
      if (status === "complete") {
        setIsPlaying(true);
        const timer = setTimeout(() => setIsPlaying(false), 5000);
        return () => clearTimeout(timer);
      }
    }, [status]);

    // Format the video title to fit in the card
    const formatTitle = (title) => {
      if (!title) return "Downloading...";
      return title.length > 50 ? title.substring(0, 47) + "..." : title;
    };

    return (
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden max-w-2xl w-full">
        {/* Header Section with Title and Status */}
        <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
          <div className="flex justify-between items-center">
            <h3 className="font-medium text-gray-800 dark:text-gray-200 truncate">
              {formatTitle(title)}
            </h3>
            <span
              className={`px-2 py-1 text-xs font-medium rounded-full ${
                status === "complete"
                  ? "bg-green-100 text-green-800 dark:bg-green-800 dark:text-green-100"
                  : status === "error"
                    ? "bg-red-100 text-red-800 dark:bg-red-800 dark:text-red-100"
                    : status === "paused"
                      ? "bg-yellow-100 text-yellow-800 dark:bg-yellow-800 dark:text-yellow-100"
                      : "bg-blue-100 text-blue-800 dark:bg-blue-800 dark:text-blue-100"
              }`}
            >
              {status === "complete"
                ? "Complete"
                : status === "error"
                  ? "Error"
                  : status === "paused"
                    ? "Paused"
                    : "Downloading"}
            </span>
          </div>
        </div>

        {/* Progress Bar */}
        <div className="px-4 pt-4">
          <div className="relative w-full h-4 bg-gray-200 dark:bg-gray-600 rounded-full overflow-hidden">
            <div
              className={`absolute left-0 top-0 h-full rounded-full transition-all duration-300 ${
                status === "complete"
                  ? "bg-green-500 dark:bg-green-600"
                  : status === "error"
                    ? "bg-red-500 dark:bg-red-600"
                    : status === "paused"
                      ? "bg-yellow-500 dark:bg-yellow-600"
                      : "bg-blue-500 dark:bg-blue-600"
              }`}
              style={{ width: `${progress}%` }}
            />
          </div>
          <div className="flex justify-between mt-1 text-xs text-gray-500 dark:text-gray-400">
            <span>{formatBytes(downloadedSize)}</span>
            <span>{progress.toFixed(1)}%</span>
            <span>{formatBytes(fileSize)}</span>
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 gap-4 p-4">
          <div className="flex items-center space-x-2">
            <Download size={16} className="text-gray-500 dark:text-gray-400" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Speed</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {formatBytes(speed)}/s
              </p>
            </div>
          </div>

          <div className="flex items-center space-x-2">
            <Clock size={16} className="text-gray-500 dark:text-gray-400" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Time Remaining
              </p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {formatTime(timeRemaining)}
              </p>
            </div>
          </div>

          <div className="flex items-center space-x-2">
            <div className="w-4 h-4 rounded-full bg-blue-500 flex items-center justify-center">
              <span className="text-white text-xs font-bold">
                {format.toUpperCase().charAt(0)}
              </span>
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Format</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {format.toUpperCase()}
              </p>
            </div>
          </div>

          <div className="flex items-center space-x-2">
            <div className="w-4 h-4 rounded-full bg-purple-500 flex items-center justify-center">
              <span className="text-white text-xs font-bold">Q</span>
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Quality
              </p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
                {quality}
              </p>
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex justify-between p-4 bg-gray-50 dark:bg-gray-700 border-t border-gray-200 dark:border-gray-600">
          <button
            onClick={onPauseResume}
            disabled={status === "complete" || status === "error"}
            className={`px-4 py-2 rounded-md text-sm font-medium
              ${
                status === "complete" || status === "error"
                  ? "bg-gray-200 text-gray-500 dark:bg-gray-600 dark:text-gray-400 cursor-not-allowed"
                  : "bg-blue-100 text-blue-700 hover:bg-blue-200 dark:bg-blue-700 dark:text-blue-100 dark:hover:bg-blue-600"
              }
            `}
          >
            {isPaused ? (
              <div className="flex items-center space-x-1">
                <Play size={16} />
                <span>Resume</span>
              </div>
            ) : (
              <div className="flex items-center space-x-1">
                <Pause size={16} />
                <span>Pause</span>
              </div>
            )}
          </button>

          <button
            onClick={onCancel}
            disabled={status === "complete"}
            className={`px-4 py-2 rounded-md text-sm font-medium
              ${
                status === "complete"
                  ? "bg-gray-200 text-gray-500 dark:bg-gray-600 dark:text-gray-400 cursor-not-allowed"
                  : "bg-red-100 text-red-700 hover:bg-red-200 dark:bg-red-700 dark:text-red-100 dark:hover:bg-red-600"
              }
            `}
          >
            <div className="flex items-center space-x-1">
              <XCircle size={16} />
              <span>Cancel</span>
            </div>
          </button>

          {status === "complete" && (
            <button className="px-4 py-2 rounded-md text-sm font-medium bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-700 dark:text-green-100 dark:hover:bg-green-600">
              <div className="flex items-center space-x-1">
                <CheckCircle size={16} />
                <span>Open File</span>
              </div>
            </button>
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="p-6 max-w-3xl mx-auto">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-white">
          Download Manager
        </h1>
        <button
          onClick={handleAddDownload}
          className="flex items-center space-x-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
        >
          <PlusCircle size={18} />
          <span>Add Download</span>
        </button>
      </div>

      <div className="space-y-6">
        {downloads.length === 0 ? (
          <div className="text-center p-8 bg-gray-50 dark:bg-gray-800 rounded-lg border border-dashed border-gray-300 dark:border-gray-700">
            <p className="text-gray-500 dark:text-gray-400">
              No active downloads
            </p>
            <button
              onClick={handleAddDownload}
              className="mt-4 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              Start a download
            </button>
          </div>
        ) : (
          downloads.map((download) => (
            <DownloadInfoCard
              key={download.id}
              downloadInfo={download}
              isPaused={download.isPaused}
              onPauseResume={() => handlePauseResume(download.id)}
              onCancel={() => handleCancel(download.id)}
            />
          ))
        )}
      </div>
    </div>
  );
};

export default DownloadManager;
