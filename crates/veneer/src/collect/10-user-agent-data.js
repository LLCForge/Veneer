if (n.userAgentData) {
  const high = await n.userAgentData
    .getHighEntropyValues(['architecture', 'bitness', 'model', 'platformVersion', 'fullVersionList'])
    .catch(() => ({}));
  out.userAgentData = {
    brands: n.userAgentData.brands.map((b) => ({ brand: b.brand, version: b.version })),
    mobile: n.userAgentData.mobile,
    platform: n.userAgentData.platform,
    architecture: high.architecture ?? null,
    bitness: high.bitness ?? null,
    model: high.model ?? null,
    platformVersion: high.platformVersion ?? null,
    fullVersionList: (high.fullVersionList ?? []).map((b) => ({ brand: b.brand, version: b.version })),
  };
}
