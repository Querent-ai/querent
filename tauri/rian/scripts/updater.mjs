import fetch from 'node-fetch';
import { getOctokit, context } from '@actions/github';

const UPDATE_TAG_NAME = 'latest';
const UPDATE_JSON_FILE = 'latest.json';

async function resolveUpdater() {
  if (!process.env.GITHUB_TOKEN) {
    throw new Error('GITHUB_TOKEN is required');
  }

  const options = { owner: context.repo.owner, repo: context.repo.repo };
  const github = getOctokit(process.env.GITHUB_TOKEN);

  // Fetch the latest tag starting with 'v'
  const { data: tags } = await github.rest.repos.listTags({
    ...options,
    per_page: 10,
    page: 1,
  });

  const tag = tags.find((t) => t.name.startsWith('v'));
  if (!tag) {
    throw new Error('No valid tag found.');
  }

  console.log(`Found tag: ${tag.name}`);

  // Fetch the latest release by the tag
  const { data: latestRelease } = await github.rest.repos.getReleaseByTag({
    ...options,
    tag: tag.name,
  });

  const updateData = {
    version: tag.name,
    notes: `Querent AI LLC Commit SHA: ${tag.commit.sha}`,
    pub_date: new Date().toISOString(),
    platforms: {
      // Populate platforms as needed
      'windows-x86_64': { signature: '', url: '' },
      'darwin-aarch64': { signature: '', url: '' },
      'darwin-x86_64': { signature: '', url: '' },
      'linux-x86_64': { signature: '', url: '' },
    },
  };

  const promises = latestRelease.assets.map(async (asset) => {
    const { name, browser_download_url } = asset;

    // Logic to update platform URLs and signatures
    if (name.endsWith('.msi.zip')) {
      updateData.platforms['windows-x86_64'].url = browser_download_url;
    } else if (name.endsWith('.msi.zip.sig')) {
      updateData.platforms['windows-x86_64'].signature = await getSignature(browser_download_url);
    } else if (name.endsWith('.app.tar.gz') && !name.includes('aarch')) {
      updateData.platforms['darwin-x86_64'].url = browser_download_url;
    } else if (name.endsWith('.app.tar.gz.sig') && !name.includes('aarch')) {
      updateData.platforms['darwin-x86_64'].signature = await getSignature(browser_download_url);
    } else if (name.endsWith('aarch64.app.tar.gz')) {
      updateData.platforms['darwin-aarch64'].url = browser_download_url;
    } else if (name.endsWith('aarch64.app.tar.gz.sig')) {
      updateData.platforms['darwin-aarch64'].signature = await getSignature(browser_download_url);
    } else if (name.endsWith('.AppImage.tar.gz')) {
      updateData.platforms['linux-x86_64'].url = browser_download_url;
    } else if (name.endsWith('.AppImage.tar.gz.sig')) {
      updateData.platforms['linux-x86_64'].signature = await getSignature(browser_download_url);
    }
  });

  await Promise.allSettled(promises);

  console.log('Resolved Update Data:', JSON.stringify(updateData, null, 2));

  // Clean up platforms without URLs
  Object.entries(updateData.platforms).forEach(([key, value]) => {
    if (!value.url) {
      console.error(`[Error]: Failed to parse release for "${key}"`);
      delete updateData.platforms[key];
    }
  });

  // Handle the latest.json asset in the update release
  const { data: updateRelease } = await github.rest.repos.getReleaseByTag({
    ...options,
    tag: UPDATE_TAG_NAME,
  });

  for (const asset of updateRelease.assets) {
    if (asset.name === UPDATE_JSON_FILE) {
      await github.rest.repos.deleteReleaseAsset({
        ...options,
        asset_id: asset.id,
      });
    }
  }

  // Upload the new latest.json file
  const response = await github.rest.repos.uploadReleaseAsset({
    ...options,
    release_id: updateRelease.id,
    name: UPDATE_JSON_FILE,
    data: Buffer.from(JSON.stringify(updateData, null, 2)),
    headers: {
      'content-type': 'application/json',
      'content-length': Buffer.byteLength(JSON.stringify(updateData, null, 2)),
    },
  });

  console.log('latest.json uploaded:', response.data.browser_download_url);
}

async function getSignature(url) {
  const response = await fetch(url, {
    method: 'GET',
    headers: { 'Content-Type': 'application/octet-stream' },
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch signature from ${url}`);
  }

  return response.text();
}

resolveUpdater().catch((error) => {
  console.error('Updater failed:', error);
  process.exit(1);
});
