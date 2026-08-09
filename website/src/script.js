const copyButtons = document.querySelectorAll("[data-copy-target]");

async function writeClipboard(text) {
  if (navigator.clipboard && window.isSecureContext) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const temporary = document.createElement("textarea");
  temporary.value = text;
  temporary.setAttribute("readonly", "");
  temporary.style.position = "fixed";
  temporary.style.opacity = "0";
  document.body.append(temporary);
  temporary.select();

  const copied = document.execCommand("copy");
  temporary.remove();

  if (!copied) {
    throw new Error("Clipboard access was unavailable");
  }
}

for (const button of copyButtons) {
  button.addEventListener("click", async () => {
    const targetId = button.dataset.copyTarget;
    const target = document.getElementById(targetId);
    const statusId = button.getAttribute("aria-describedby");
    const status = statusId ? document.getElementById(statusId) : null;

    if (!target) {
      return;
    }

    const originalLabel = button.textContent;

    try {
      await writeClipboard(target.textContent.trim());
      button.textContent = "Copied";
      if (status) {
        status.textContent = "Copied to clipboard.";
      }
    } catch {
      button.textContent = "Select";
      if (status) {
        status.textContent = "Clipboard access is unavailable. Select the command to copy it.";
      }
    }

    window.setTimeout(() => {
      button.textContent = originalLabel;
      if (status) {
        status.textContent = "";
      }
    }, 2400);
  });
}
