import fs from 'fs';
import path from 'path';
import { getOctokit, context } from '@actions/github';

const UPDATE_JSON_FILE = 'update.json';
const ARTIFACTS_DIR = 'artifacts';

async function uploadReleaseFiles() {
  if (!process.env.GITHUB_TOKEN) {
    throw new Error('GITHUB_TOKEN is required');
  }
  if(!process.env.RELEASE_ID && !process.env.TAG_NAME) {
    throw new Error('RELEASE_ID or TAG_NAME is required');
  }
  const github = getOctokit(process.env.GITHUB_TOKEN);
  const updateData = JSON.parse(fs.readFileSync(UPDATE_JSON_FILE, 'utf8'));

  for (const platform in updateData.platforms) {
    const platformData = updateData.platforms[platform];
    const fileName = platformData.url.split('/').pop();
    const filePath = path.join(ARTIFACTS_DIR, platform, fileName);

    if (fs.existsSync(filePath)) {
      const data = fs.readFileSync(filePath);
      console.log(`Uploading ${fileName} for platform ${platform}...`);

      if (!process.env.RELEASE_ID) {
        await github.rest.repos.uploadReleaseAsset({
          owner: context.repo.owner,
          repo: context.repo.repo,
          release_id: process.env.RELEASE_ID,
          name: fileName,
          data,
          headers: {
            'content-type': 'application/octet-stream',
            'content-length': data.length,
          },
        });
      } else {
        await github.rest.repos.uploadReleaseAsset({
          owner: context.repo.owner,
          repo: context.repo.repo,
          tag_name: process.env.TAG_NAME,
          name: fileName,
          data,
          headers: {
            'content-type': 'application/octet-stream',
            'content-length': data.length,
          },
        });
      }
    } else {
      console.error(`[Error]: File ${fileName} not found for platform ${platform}`);
    }
  }
}

uploadReleaseFiles().catch((error) => {
  console.error('Failed to upload release files:', error);
  process.exit(1);
});
