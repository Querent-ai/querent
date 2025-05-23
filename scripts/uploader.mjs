import fs from 'fs';
import path from 'path';
import { getOctokit, context } from '@actions/github';

const UPDATE_JSON_FILE = 'update.json';
const ARTIFACTS_DIR = 'artifacts';

async function uploadReleaseFiles() {
  if (!process.env.GITHUB_TOKEN) {
    throw new Error('GITHUB_TOKEN is required');
  }
  if(!process.env.TAG_NAME) {
    throw new Error('TAG_NAME is required');
  }
  const github = getOctokit(process.env.GITHUB_TOKEN);
  const updateData = JSON.parse(fs.readFileSync(UPDATE_JSON_FILE, 'utf8'));
  const release = await github.rest.repos.getReleaseByTag({
    owner: context.repo.owner,
    repo: context.repo.repo,
    tag: process.env.TAG_NAME,
  });
  if (!release.data) {
    throw new Error(`Release with tag ${process.env.TAG_NAME} not found`);
  }
  for (const platform in updateData.platforms) {
    const platformData = updateData.platforms[platform];
    const fileName = platformData.url.split('/').pop();
    let filenameLocal = fileName;
    if (platform === 'darwin-aarch64') {
      // Remove 'aarch64-' prefix
      filenameLocal = fileName.slice(8);
    }
    const filePath = path.join(ARTIFACTS_DIR, platform, filenameLocal);

    if (fs.existsSync(filePath)) {
      const data = fs.readFileSync(filePath);
      console.log(`Uploading ${fileName} for platform ${platform}...`);
        await github.rest.repos.uploadReleaseAsset({
          owner: context.repo.owner,
          repo: context.repo.repo,
          release_id: release.data.id,
          name: fileName,
          data,
          headers: {
            'content-type': 'application/octet-stream',
            'content-length': data.length,
          },
        });
    } else {
      console.error(`[Error]: File ${fileName} not found for platform ${platform}`);
    }
  }
}

uploadReleaseFiles().catch((error) => {
  console.error('Failed to upload release files:', error);
  process.exit(1);
});
