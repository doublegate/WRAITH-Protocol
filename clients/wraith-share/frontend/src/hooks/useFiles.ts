// useFiles Hook - Convenience hook for file operations

import { useCallback, useMemo } from 'react';
import { useFileStore } from '../stores/fileStore';
import { useGroupStore } from '../stores/groupStore';
import { useUiStore } from '../stores/uiStore';

export function useFiles() {
  const { selectedGroupId, groupInfos } = useGroupStore();
  const {
    files,
    selectedFileId,
    uploads,
    loading,
    error,
    sortBy,
    sortOrder,
    searchQuery,
    getSortedFiles,
    fetchFiles,
    uploadFile,
    downloadFile,
    deleteFile,
    selectFile,
    setSearchQuery,
    setSortBy,
    setSortOrder,
    clearError,
  } = useFileStore();

  const { addToast, openModal } = useUiStore();

  const groupInfo = selectedGroupId ? groupInfos.get(selectedGroupId) : null;
  const canUpload =
    groupInfo?.my_role === 'admin' || groupInfo?.my_role === 'write';
  const canDelete =
    groupInfo?.my_role === 'admin' || groupInfo?.my_role === 'write';

  const sortedFiles = useMemo(() => getSortedFiles(), [getSortedFiles]);

  const selectedFile = useMemo(
    () => files.find((f) => f.id === selectedFileId),
    [files, selectedFileId]
  );

  const handleUpload = useCallback(
    async (file: File) => {
      if (!selectedGroupId || !canUpload) return;

      try {
        await uploadFile(selectedGroupId, file);
        addToast('success', `Uploaded ${file.name}`);
      } catch (err) {
        addToast('error', (err as Error).message);
      }
    },
    [selectedGroupId, canUpload, uploadFile, addToast]
  );

  const handleDownload = useCallback(
    async (fileId: string, fileName: string) => {
      try {
        await downloadFile(fileId, fileName);
        addToast('success', `Downloaded ${fileName}`);
      } catch (err) {
        addToast('error', (err as Error).message);
      }
    },
    [downloadFile, addToast]
  );

  const handleDelete = useCallback(
    async (fileId: string) => {
      const file = files.find((f) => f.id === fileId);
      if (!file || !canDelete) return;

      if (!confirm(`Delete "${file.name}"?`)) return;

      try {
        await deleteFile(fileId);
        addToast('success', `Deleted ${file.name}`);
      } catch (err) {
        addToast('error', (err as Error).message);
      }
    },
    [files, canDelete, deleteFile, addToast]
  );

  const openUploadModal = useCallback(() => {
    if (canUpload) {
      openModal('fileUpload');
    }
  }, [canUpload, openModal]);

  const openShareModal = useCallback(
    (fileId: string) => {
      openModal('shareLink', fileId);
    },
    [openModal]
  );

  return {
    // State
    files: sortedFiles,
    selectedFile,
    uploads,
    loading,
    error,
    sortBy,
    sortOrder,
    searchQuery,
    canUpload,
    canDelete,

    // Actions
    refresh: () => selectedGroupId && fetchFiles(selectedGroupId),
    upload: handleUpload,
    download: handleDownload,
    delete: handleDelete,
    select: selectFile,
    search: setSearchQuery,
    setSortBy,
    setSortOrder,
    clearError,
    openUploadModal,
    openShareModal,
  };
}

export function useFile(fileId?: string) {
  const { files, versions, shareLinks, fetchVersions, fetchShareLinks, restoreVersion } =
    useFileStore();
  const { addToast } = useUiStore();

  const file = useMemo(
    () => files.find((f) => f.id === fileId),
    [files, fileId]
  );

  const handleRestore = useCallback(
    async (version: number) => {
      if (!fileId) return;
      if (!confirm(`Restore to version ${version}?`)) return;

      try {
        await restoreVersion(fileId, version);
        addToast('success', `Restored to version ${version}`);
      } catch (err) {
        addToast('error', (err as Error).message);
      }
    },
    [fileId, restoreVersion, addToast]
  );

  return {
    file,
    versions,
    shareLinks,
    refreshVersions: () => fileId && fetchVersions(fileId),
    refreshShareLinks: () => fileId && fetchShareLinks(fileId),
    restore: handleRestore,
  };
}
