import fs from 'fs';
import path from 'path';

const UPDATE_JSON_FILE = 'update.json';
const ARTIFACTS_DIR = 'artifacts';

const updateData = {
  version: process.env.ASSET_VERSION || 'unknown',
  notes: `Querent AI LLC Commit SHA for R!AN: ${process.env.GITHUB_SHA || 'unknown'}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': { signature: '', url: '' },
    'darwin-aarch64': { signature: '', url: '' },
    'darwin-x86_64': { signature: '', url: '' },
    'linux-x86_64': { signature: '', url: '' },
  },
};

function getFileNameAndSignature(directory) {
  const files = fs.readdirSync(directory);
  let fileName = '';
  let signaturePath = '';

  files.forEach(file => {
    if (file.endsWith('.msi.zip') || file.endsWith('.app.tar.gz') || file.endsWith('.AppImage.tar.gz')) {
      fileName = file;
    } else if (file.endsWith('.msi.zip.sig') || file.endsWith('.app.tar.gz.sig') || file.endsWith('.AppImage.tar.gz.sig')) {
      signaturePath = path.join(directory, file);
    }
  });

  return { fileName, signaturePath };
}

function readSignature(signaturePath) {
  return fs.existsSync(signaturePath) ? fs.readFileSync(signaturePath, 'utf8') : '';
}

// Resolve artifact paths and signatures
['windows-x86_64', 'darwin-aarch64', 'darwin-x86_64', 'linux-x86_64'].forEach(platformKey => {
  const { fileName, signaturePath } = getFileNameAndSignature(path.join(ARTIFACTS_DIR, platformKey));
  
  if (fileName) {
    updateData.platforms[platformKey].url = `https://github.com/querent-ai/distribution/releases/download/${process.env.ASSET_VERSION}/${fileName}`;
    updateData.platforms[platformKey].signature = readSignature(signaturePath);
  } else {
    console.error(`[Error]: No artifact found for "${platformKey}"`);
  }
});

// Remove platforms with no URLs
Object.entries(updateData.platforms).forEach(([key, value]) => {
  if (!value.url) {
    delete updateData.platforms[key];
  }
});

// Write update.json to the root directory
fs.writeFileSync(UPDATE_JSON_FILE, JSON.stringify(updateData, null, 2));

console.log('update.json generated successfully:', path.resolve(UPDATE_JSON_FILE));
