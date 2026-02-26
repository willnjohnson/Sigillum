export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}

export function showModal(
  modalOverlay: HTMLElement,
  modalTitle: HTMLElement,
  modalContent: HTMLElement,
  title: string,
  content: string
) {
  modalTitle.textContent = title;
  modalContent.innerHTML = content;
  modalOverlay.classList.remove("hidden");
}

export function hideModal(modalOverlay: HTMLElement) {
  modalOverlay.classList.add("hidden");
}

export async function readFileAsBytes(file: File): Promise<number[]> {
  const data = await file.arrayBuffer();
  return Array.from(new Uint8Array(data));
}

export function createDownloadLink(data: number[], filename: string): void {
  const blob = new Blob([new Uint8Array(data)], { type: "application/pdf" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export function setButtonLoading(button: HTMLButtonElement, loading: boolean, loadingText: string) {
  button.disabled = loading;
  button.innerHTML = loading ? `<span class="loading"></span>${loadingText}` : (button.textContent || "");
}

export function resetButton(button: HTMLButtonElement, originalText: string) {
  button.disabled = false;
  button.textContent = originalText;
}

export function getElement<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

export function toggleDropZoneContent(dropZone: HTMLElement, showContent: boolean) {
  const content = dropZone.querySelector(".drop-zone-content");
  if (content) content.classList.toggle("hidden", !showContent);
}

export function setupDropZone(
  dropZone: HTMLElement,
  fileInput: HTMLInputElement,
  onFileSelect: (file: File) => void,
  onFileInfo: HTMLElement | null,
  fileNameEl: HTMLElement | null,
  fileSizeEl: HTMLElement | null
) {
  dropZone.addEventListener("click", () => fileInput.click());

  dropZone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropZone.classList.add("drag-over");
  });

  dropZone.addEventListener("dragleave", () => {
    dropZone.classList.remove("drag-over");
  });

  dropZone.addEventListener("drop", (e) => {
    e.preventDefault();
    dropZone.classList.remove("drag-over");
    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      handleFileSelect(files[0], onFileSelect, onFileInfo, fileNameEl, fileSizeEl);
    }
  });

  fileInput.addEventListener("change", () => {
    if (fileInput.files && fileInput.files.length > 0) {
      handleFileSelect(fileInput.files[0], onFileSelect, onFileInfo, fileNameEl, fileSizeEl);
    }
  });
}

function handleFileSelect(
  file: File,
  onFileSelect: (file: File) => void,
  onFileInfo: HTMLElement | null,
  fileNameEl: HTMLElement | null,
  fileSizeEl: HTMLElement | null
) {
  if (!file.type.includes("pdf")) {
    return;
  }
  onFileSelect(file);
  if (onFileInfo) onFileInfo.classList.remove("hidden");
  if (fileNameEl) fileNameEl.textContent = file.name;
  if (fileSizeEl) fileSizeEl.textContent = formatFileSize(file.size);
  const dropZone = onFileInfo?.parentElement as HTMLElement | null;
  if (dropZone) toggleDropZoneContent(dropZone, false);
}

export function showTemporarySuccess(
  modalOverlay: HTMLElement,
  modalTitle: HTMLElement,
  modalContent: HTMLElement,
  message: string,
  duration: number = 2000
) {
  showModal(modalOverlay, modalTitle, modalContent, "Success", `<p class="message success">${message}</p>`);
  setTimeout(() => hideModal(modalOverlay), duration);
}

export function showError(
  modalOverlay: HTMLElement,
  modalTitle: HTMLElement,
  modalContent: HTMLElement,
  message: string
) {
  showModal(modalOverlay, modalTitle, modalContent, "Error", `<p class="message error">${message}</p>`);
}
