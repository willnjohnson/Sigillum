import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import {
  showModal,
  hideModal,
  readFileAsBytes,
  createDownloadLink,
  setButtonLoading,
  resetButton,
  getElement,
  setupDropZone,
  showTemporarySuccess,
  showError,
} from "./utils";

interface SignPdfRequest {
  pdf_data: number[];
  name: string;
  extra: string;
}

interface SignPdfResponse {
  signed_pdf: number[];
  signature_info: {
    signer_name: string;
    timestamp: string;
    extra: string;
    signature: string;
  };
}

interface VerifyPdfResponse {
  is_signed: boolean;
  signature_info: {
    signer_name: string;
    timestamp: string;
    extra: string;
    signature: string;
  } | null;
  message: string;
}

const state = {
  hasKey: false,
  currentPublicKey: "",
  selectedFile: null as File | null,
  signedPdfData: null as number[] | null,
  verifySelectedFile: null as File | null,
  currentTab: "sign-section",
};

const elements = {
  btnGenerateKey: getElement<HTMLButtonElement>("btn-generate-key"),
  btnImportKey: getElement<HTMLButtonElement>("btn-import-key"),
  btnExportKey: getElement<HTMLButtonElement>("btn-export-key"),
  keySection: getElement<HTMLElement>("key-section"),
  publicKeyContent: getElement<HTMLElement>("public-key-content"),
  noKeyMessage: getElement<HTMLElement>("no-key-message"),
  signForm: getElement<HTMLElement>("sign-form"),
  signerNameInput: getElement<HTMLInputElement>("signer-name"),
  extraTextInput: getElement<HTMLInputElement>("extra-text"),
  dropZone: getElement<HTMLElement>("drop-zone"),
  fileInput: getElement<HTMLInputElement>("file-input"),
  fileInfo: getElement<HTMLElement>("file-info"),
  fileName: getElement<HTMLElement>("file-name"),
  fileSize: getElement<HTMLElement>("file-size"),
  btnSign: getElement<HTMLButtonElement>("btn-sign"),
  resultSection: getElement<HTMLElement>("result-section"),
  resultName: getElement<HTMLElement>("result-name"),
  resultTimestamp: getElement<HTMLElement>("result-timestamp"),
  resultExtra: getElement<HTMLElement>("result-extra"),
  resultSignature: getElement<HTMLElement>("result-signature"),
  btnDownload: getElement<HTMLButtonElement>("btn-download"),
  modalOverlay: getElement<HTMLElement>("modal-overlay"),
  modal: getElement<HTMLElement>("modal"),
  modalTitle: getElement<HTMLElement>("modal-title"),
  modalContent: getElement<HTMLElement>("modal-content"),
  modalClose: getElement<HTMLButtonElement>("modal-close"),
  tabSign: getElement<HTMLButtonElement>("tab-sign"),
  tabVerify: getElement<HTMLButtonElement>("tab-verify"),
  signSection: getElement<HTMLElement>("sign-section"),
  verifySection: getElement<HTMLElement>("verify-section"),
  verifyDropZone: getElement<HTMLElement>("verify-drop-zone"),
  verifyFileInput: getElement<HTMLInputElement>("verify-file-input"),
  verifyFileInfo: getElement<HTMLElement>("verify-file-info"),
  verifyFileName: getElement<HTMLElement>("verify-file-name"),
  verifyFileSize: getElement<HTMLElement>("verify-file-size"),
  btnVerify: getElement<HTMLButtonElement>("btn-verify"),
  verifyResult: getElement<HTMLElement>("verify-result"),
  verifySuccess: getElement<HTMLElement>("verify-success"),
  verifyError: getElement<HTMLElement>("verify-error"),
  verifyMessage: getElement<HTMLElement>("verify-message"),
  verifyErrorMessage: getElement<HTMLElement>("verify-error-message"),
  verifyDetails: getElement<HTMLElement>("verify-details"),
  verifyName: getElement<HTMLElement>("verify-name"),
  verifyTimestamp: getElement<HTMLElement>("verify-timestamp"),
  verifyExtra: getElement<HTMLElement>("verify-extra"),
  verifySignature: getElement<HTMLElement>("verify-signature"),
};

const { modalOverlay, modalTitle, modalContent, btnSign, btnVerify } = elements;

async function checkKeyStatus() {
  try {
    state.hasKey = await invoke<boolean>("has_key");
    if (state.hasKey) {
      state.currentPublicKey = await invoke<string>("get_public_key");
      updateKeyUI(true);
    } else {
      updateKeyUI(false);
    }
  } catch (error) {
    console.error("Failed to check key status:", error);
  }
}

function updateKeyUI(hasKey: boolean) {
  const { btnGenerateKey, btnImportKey, btnExportKey, keySection, publicKeyContent, noKeyMessage, signForm } = elements;
  
  btnGenerateKey.classList.toggle("hidden", hasKey);
  btnImportKey.classList.toggle("hidden", hasKey);
  btnExportKey.classList.toggle("hidden", !hasKey);
  keySection.classList.toggle("hidden", !hasKey);
  publicKeyContent.textContent = hasKey ? state.currentPublicKey : "No key loaded";
  noKeyMessage.classList.toggle("hidden", hasKey);
  signForm.classList.toggle("hidden", !hasKey);
}

function updateSignButton() {
  btnSign.disabled = !state.selectedFile || !elements.signerNameInput.value.trim();
}

function updateVerifyButton() {
  btnVerify.disabled = !state.verifySelectedFile;
}

function displaySignResult(response: SignPdfResponse) {
  const { resultName, resultTimestamp, resultExtra, resultSignature, resultSection } = elements;
  
  resultName.textContent = response.signature_info.signer_name;
  resultTimestamp.textContent = response.signature_info.timestamp;
  resultExtra.textContent = response.signature_info.extra || "(none)";
  resultSignature.textContent = response.signature_info.signature;
  resultSection.classList.remove("hidden");
  resultSection.scrollIntoView({ behavior: "smooth" });
}

function clearSignForm() {
  const { signerNameInput, extraTextInput, fileInput, resultSection, fileInfo } = elements;
  
  signerNameInput.value = "";
  extraTextInput.value = "";
  fileInput.value = "";
  state.selectedFile = null;
  resultSection.classList.add("hidden");
  fileInfo.classList.add("hidden");
  const dropContent = elements.dropZone.querySelector(".drop-zone-content");
  if (dropContent) dropContent.classList.remove("hidden");
  btnSign.disabled = true;
}

function clearVerifyForm() {
  const { verifyFileInput, verifyResult, verifySuccess, verifyError, verifyDetails, resultSection, verifyFileInfo } = elements;
  
  verifyFileInput.value = "";
  state.verifySelectedFile = null;
  verifyResult.classList.add("hidden");
  verifySuccess.classList.add("hidden");
  verifyError.classList.add("hidden");
  verifyDetails.classList.add("hidden");
  resultSection.classList.add("hidden");
  verifyFileInfo.classList.add("hidden");
  const dropContent = elements.verifyDropZone.querySelector(".drop-zone-content");
  if (dropContent) dropContent.classList.remove("hidden");
  btnVerify.disabled = true;
}

function switchTab(tabId: string) {
  state.currentTab = tabId;
  const isSignTab = tabId === "sign-section";
  
  elements.tabSign.classList.toggle("active", isSignTab);
  elements.tabVerify.classList.toggle("active", !isSignTab);
  elements.signSection.classList.toggle("hidden", !isSignTab);
  elements.verifySection.classList.toggle("hidden", isSignTab);
  elements.keySection.classList.toggle("hidden", !isSignTab || !state.hasKey);
  
  if (isSignTab) {
    clearSignForm();
  } else {
    clearVerifyForm();
  }
}

async function generateKeypair() {
  showModal(modalOverlay, modalTitle, modalContent, "Generate Keypair", '<p class="message info">Generating RSA keypair...</p>');
  
  try {
    const publicKey = await invoke<string>("generate_keypair");
    state.currentPublicKey = publicKey;
    state.hasKey = true;
    hideModal(modalOverlay);
    updateKeyUI(true);
    showTemporarySuccess(modalOverlay, modalTitle, modalContent, "Keypair generated successfully!");
  } catch (error) {
    hideModal(modalOverlay);
    showError(modalOverlay, modalTitle, modalContent, `Failed to generate keypair: ${error}`);
  }
}

async function importKey(privateKeyPem: string, publicKeyPem: string) {
  try {
    const publicKey = await invoke<string>("import_key", { privateKeyPem, publicKeyPem });
    state.currentPublicKey = publicKey;
    state.hasKey = true;
    hideModal(modalOverlay);
    updateKeyUI(true);
    showTemporarySuccess(modalOverlay, modalTitle, modalContent, "Key imported successfully!");
  } catch (error) {
    hideModal(modalOverlay);
    showError(modalOverlay, modalTitle, modalContent, `Failed to import key: ${error}`);
  }
}

async function exportKey() {
  try {
    const privateKey = await invoke<string>("export_key");
    createDownloadLink(Array.from(privateKey).map(c => c.charCodeAt(0)), "private_key.pem");
    showTemporarySuccess(modalOverlay, modalTitle, modalContent, "Private key exported!");
  } catch (error) {
    showError(modalOverlay, modalTitle, modalContent, `Failed to export key: ${error}`);
  }
}

async function signPdf() {
  if (!state.selectedFile || !elements.signerNameInput.value.trim()) {
    showError(modalOverlay, modalTitle, modalContent, "Please enter your name and select a PDF file.");
    return;
  }

  try {
    setButtonLoading(btnSign, true, "Signing...");
    
    const pdfBytes = await readFileAsBytes(state.selectedFile);
    const request: SignPdfRequest = {
      pdf_data: pdfBytes,
      name: elements.signerNameInput.value.trim(),
      extra: elements.extraTextInput.value.trim(),
    };
    
    const response = await invoke<SignPdfResponse>("sign_pdf", { request });
    state.signedPdfData = response.signed_pdf;
    
    displaySignResult(response);
    resetButton(btnSign, "Sign PDF");
  } catch (error) {
    showError(modalOverlay, modalTitle, modalContent, `Failed to sign PDF: ${error}`);
    resetButton(btnSign, "Sign PDF");
  }
}

async function downloadSignedPdf() {
  if (!state.signedPdfData) return;
  
  try {
    const defaultFileName = state.selectedFile
      ? state.selectedFile.name.replace(/\.pdf$/i, "_SIGNED.pdf")
      : "signed_document.pdf";
    
    const filePath = await save({
      defaultPath: defaultFileName,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    
    if (filePath) {
      await writeFile(filePath, new Uint8Array(state.signedPdfData));
      showTemporarySuccess(modalOverlay, modalTitle, modalContent, "PDF saved successfully!");
    }
  } catch (error) {
    showError(modalOverlay, modalTitle, modalContent, `Failed to save PDF: ${error}`);
  }
}

async function verifyPdf() {
  if (!state.verifySelectedFile) return;

  try {
    setButtonLoading(btnVerify, true, "Verifying...");
    
    const pdfBytes = await readFileAsBytes(state.verifySelectedFile);
    const response = await invoke<VerifyPdfResponse>("verify_pdf", { pdfData: pdfBytes });
    
    elements.verifyResult.classList.remove("hidden");
    
    if (response.is_signed && response.signature_info) {
      elements.verifySuccess.classList.remove("hidden");
      elements.verifyError.classList.add("hidden");
      elements.verifyDetails.classList.remove("hidden");
      
      elements.verifyMessage.textContent = response.message;
      elements.verifyName.textContent = response.signature_info.signer_name;
      elements.verifyTimestamp.textContent = response.signature_info.timestamp;
      elements.verifyExtra.textContent = response.signature_info.extra || "(none)";
      elements.verifySignature.textContent = response.signature_info.signature;
    } else {
      elements.verifySuccess.classList.add("hidden");
      elements.verifyError.classList.remove("hidden");
      elements.verifyDetails.classList.add("hidden");
      elements.verifyErrorMessage.textContent = response.message;
    }
    
    resetButton(btnVerify, "Verify PDF");
    elements.verifyResult.scrollIntoView({ behavior: "smooth" });
  } catch (error) {
    showError(modalOverlay, modalTitle, modalContent, `Failed to verify PDF: ${error}`);
    resetButton(btnVerify, "Verify PDF");
  }
}

function initKeyButtons() {
  elements.btnGenerateKey.addEventListener("click", () => {
    showModal(modalOverlay, modalTitle, modalContent, "Generate Keypair", `
      <p>Generate a new RSA keypair for signing PDFs?</p>
      <div class="modal-actions">
        <button id="modal-cancel" class="menu-btn" style="background: #64748b;">Cancel</button>
        <button id="modal-confirm" class="menu-btn">Generate</button>
      </div>
    `);
    getElement<HTMLButtonElement>("modal-cancel").addEventListener("click", () => hideModal(modalOverlay));
    getElement<HTMLButtonElement>("modal-confirm").addEventListener("click", generateKeypair);
  });

  elements.btnImportKey.addEventListener("click", () => {
    showModal(modalOverlay, modalTitle, modalContent, "Import Key", `
      <div class="form-group">
        <label>Private Key (PEM):</label>
        <textarea id="import-private-key" class="key-input" placeholder="-----BEGIN PRIVATE KEY-----"></textarea>
      </div>
      <div class="form-group">
        <label>Public Key (PEM):</label>
        <textarea id="import-public-key" class="key-input" placeholder="-----BEGIN PUBLIC KEY-----"></textarea>
      </div>
      <div class="modal-actions">
        <button id="modal-cancel" class="menu-btn" style="background: #64748b;">Cancel</button>
        <button id="modal-import" class="menu-btn">Import</button>
      </div>
    `);
    getElement<HTMLButtonElement>("modal-cancel").addEventListener("click", () => hideModal(modalOverlay));
    getElement<HTMLButtonElement>("modal-import").addEventListener("click", () => {
      const privateKey = getElement<HTMLTextAreaElement>("import-private-key").value.trim();
      const publicKey = getElement<HTMLTextAreaElement>("import-public-key").value.trim();
      
      if (!privateKey || !publicKey) {
        showError(modalOverlay, modalTitle, modalContent, "Please provide both private and public keys.");
        return;
      }
      importKey(privateKey, publicKey);
    });
  });

  elements.btnExportKey.addEventListener("click", exportKey);
}

function initDropZones() {
  setupDropZone(
    elements.dropZone,
    elements.fileInput,
    (file) => {
      state.selectedFile = file;
      updateSignButton();
    },
    elements.fileInfo,
    elements.fileName,
    elements.fileSize
  );

  setupDropZone(
    elements.verifyDropZone,
    elements.verifyFileInput,
    (file) => {
      state.verifySelectedFile = file;
      updateVerifyButton();
    },
    elements.verifyFileInfo,
    elements.verifyFileName,
    elements.verifyFileSize
  );
}

function initEventListeners() {
  elements.tabSign.addEventListener("click", () => switchTab("sign-section"));
  elements.tabVerify.addEventListener("click", () => switchTab("verify-section"));
  
  elements.signerNameInput.addEventListener("input", updateSignButton);
  elements.btnSign.addEventListener("click", signPdf);
  elements.btnDownload.addEventListener("click", downloadSignedPdf);
  elements.btnVerify.addEventListener("click", verifyPdf);
  
  elements.modalClose.addEventListener("click", () => hideModal(modalOverlay));
  modalOverlay.addEventListener("click", (e) => {
    if (e.target === modalOverlay) hideModal(modalOverlay);
  });
}

function init() {
  initKeyButtons();
  initDropZones();
  initEventListeners();
  checkKeyStatus();
}

window.addEventListener("DOMContentLoaded", init);
