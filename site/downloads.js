(() => {
  const releaseBase = "https://github.com/resistdesign/ittsy/releases/download/v0.3.0";
  const releasesUrl = "https://github.com/resistdesign/ittsy/releases/latest";
  const downloads = {
    macArm: `${releaseBase}/ittsy-v0.3.0-aarch64-apple-darwin.zip`,
    macIntel: `${releaseBase}/ittsy-v0.3.0-x86_64-apple-darwin.zip`,
    windows: `${releaseBase}/ittsy-v0.3.0-x86_64-pc-windows-msvc.zip`,
    linux: `${releaseBase}/ittsy-v0.3.0-x86_64-unknown-linux-gnu.tar.gz`,
  };

  const platformName = document.querySelector("#download-platform");
  const platformDownloads = document.querySelector("#platform-downloads");
  const primaryDownloads = document.querySelectorAll("[data-primary-download]");

  function setDownloads(name, links, primaryOverride) {
    platformName.textContent = name;
    platformDownloads.replaceChildren(
      ...links.map(({ label, url }) => {
        const link = document.createElement("a");
        link.href = url;
        link.textContent = label;
        return link;
      }),
    );

    const primary =
      primaryOverride || links[0] || { label: "View releases", url: releasesUrl };
    primaryDownloads.forEach((link) => {
      link.href = primary.url;
      link.setAttribute("aria-label", `Download ittsy: ${primary.label}`);
    });
  }

  function platform() {
    const value = [
      navigator.userAgentData?.platform,
      navigator.platform,
      navigator.userAgent,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();

    if (value.includes("mac")) return "mac";
    if (value.includes("win")) return "windows";
    if (value.includes("linux")) return "linux";
    return "unknown";
  }

  async function macArchitecture() {
    if (!navigator.userAgentData?.getHighEntropyValues) return "unknown";

    try {
      const values = await navigator.userAgentData.getHighEntropyValues(["architecture"]);
      if (values.architecture === "arm") return "arm";
      if (values.architecture === "x86") return "intel";
    } catch {
      // Fall through to offering both Mac builds.
    }

    return "unknown";
  }

  async function initializeDownloads() {
    switch (platform()) {
      case "windows":
        setDownloads("Windows", [{ label: "Download .zip", url: downloads.windows }]);
        break;
      case "linux":
        setDownloads("Linux", [{ label: "Download .tar.gz", url: downloads.linux }]);
        break;
      case "mac": {
        const architecture = await macArchitecture();
        const macLinks = [
          { label: "Apple Silicon", url: downloads.macArm },
          { label: "Intel Mac", url: downloads.macIntel },
        ];
        if (architecture === "intel") macLinks.reverse();
        const primary =
          architecture === "unknown"
            ? { label: "Choose a Mac build", url: releasesUrl }
            : undefined;
        setDownloads("macOS", macLinks, primary);
        break;
      }
      default:
        setDownloads("Native desktop app", [
          { label: "Choose your platform", url: releasesUrl },
        ]);
    }
  }

  void initializeDownloads();
})();
