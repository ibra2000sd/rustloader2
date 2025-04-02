// src/hooks/useOptimizedDownloads.js
import { useState, useEffect, useRef, useCallback } from 'react';

/**
 * Custom hook for efficiently managing downloads and optimizing UI rendering
 * Handles batch updates and prevents UI jank during high-frequency progress updates
 */
export function useOptimizedDownloads() {
  const [downloads, setDownloads] = useState([]);
  const [isInitialized, setIsInitialized] = useState(false);
  const batchedUpdatesRef = useRef(new Map());
  const rafIdRef = useRef(null);
  const updateDeferredMs = useRef(0);
  
  // Batch multiple downloads updates to prevent excessive re-renders
  const processBatchUpdate = useCallback(() => {
    if (batchedUpdatesRef.current.size === 0) return;
    
    setDownloads(currentDownloads => {
      const updatedDownloads = [...currentDownloads];
      
      // Apply all batched updates at once
      batchedUpdatesRef.current.forEach((updateData, id) => {
        const existingIndex = updatedDownloads.findIndex(d => d.id === id);
        
        if (existingIndex >= 0) {
          // Update existing download
          updatedDownloads[existingIndex] = {
            ...updatedDownloads[existingIndex],
            ...updateData
          };
        } else {
          // Add new download
          updatedDownloads.push({
            id,
            ...updateData
          });
        }
      });
      
      // Clear processed updates
      batchedUpdatesRef.current.clear();
      
      // Sort downloads: active first, then paused, then completed/failed
      return updatedDownloads.sort((a, b) => {
        // Helper function to get priority
        const getStatusPriority = (status) => {
          switch(status) {
            case 'downloading': return 0;
            case 'paused': return 1;
            case 'queued': return 2;
            case 'complete': return 3;
            case 'error': return 4;
            case 'cancelled': return 5;
            default: return 6;
          }
        };
        
        const priorityA = getStatusPriority(a.status);
        const priorityB = getStatusPriority(b.status);
        
        if (priorityA !== priorityB) {
          return priorityA - priorityB;
        }
        
        // Secondary sort by progress (descending)
        return b.progress - a.progress;
      });
    });
    
    // Clear the animation frame ID
    rafIdRef.current = null;
  }, []);
  
  // Queue a batched update with smart throttling
  const queueBatchUpdate = useCallback(() => {
    if (rafIdRef.current) return; // Update already queued
    
    // Adjust frame rate based on the number of downloads
    const downloadCount = batchedUpdatesRef.current.size;
    const adjustedDelay = downloadCount > 10 
      ? 200  // More downloads = lower frame rate to prevent jank
      : downloadCount > 5 
        ? 100  // Medium number of downloads
        : 50;  // Few downloads = higher frame rate
    
    // If last update was recent, delay this update
    const now = performance.now();
    const timeUntilNextFrame = Math.max(0, updateDeferredMs.current - (now - updateDeferredMs.current));
    
    // Queue the update with adaptive timing
    rafIdRef.current = window.setTimeout(() => {
      requestAnimationFrame(() => {
        processBatchUpdate();
        updateDeferredMs.current = performance.now() + adjustedDelay;
      });
    }, timeUntilNextFrame);
  }, [processBatchUpdate]);
  
  // Update a single download (batched)
  const updateDownload = useCallback((id, data) => {
    batchedUpdatesRef.current.set(id, data);
    queueBatchUpdate();
  }, [queueBatchUpdate]);
  
  // Remove a download
  const removeDownload = useCallback((id) => {
    batchedUpdatesRef.current.delete(id);
    setDownloads(currentDownloads => 
      currentDownloads.filter(download => download.id !== id)
    );
  }, []);
  
  // Clear all downloads
  const clearDownloads = useCallback(() => {
    batchedUpdatesRef.current.clear();
    setDownloads([]);
  }, []);
  
  // Batch update multiple downloads at once
  const updateDownloadsBatch = useCallback((updatesArray) => {
    if (!Array.isArray(updatesArray) || updatesArray.length === 0) return;
    
    // Process all updates in batch
    updatesArray.forEach(update => {
      if (update && update.id) {
        batchedUpdatesRef.current.set(update.id, update);
      }
    });
    
    queueBatchUpdate();
  }, [queueBatchUpdate]);
  
  // Initialize and load existing downloads
  useEffect(() => {
    if (isInitialized) return;
    
    const initializeDownloads = async () => {
      try {
        if (window.__TAURI__) {
          // If using Tauri, fetch existing downloads from backend
          const { invoke } = window.__TAURI__;
          const existingDownloads = await invoke('list_downloads');
          
          if (Array.isArray(existingDownloads) && existingDownloads.length > 0) {
            setDownloads(existingDownloads);
          }
          
          // Listen for batch progress updates
          const { listen } = window.__TAURI__;
          await listen('download-progress-batch', (event) => {
            if (Array.isArray(event.payload)) {
              updateDownloadsBatch(event.payload);
            }
          });
        }
        
        setIsInitialized(true);
      } catch (error) {
        console.error('Failed to initialize downloads:', error);
      }
    };
    
    initializeDownloads();
    
    // Clean up
    return () => {
      if (rafIdRef.current) {
        clearTimeout(rafIdRef.current);
      }
    };
  }, [isInitialized, updateDownloadsBatch]);
  
  return {
    downloads,
    updateDownload,
    updateDownloadsBatch,
    removeDownload,
    clearDownloads,
    isInitialized,
  };
}