import fs from 'fs';
import path from 'path';

const UPDATE_JSON_FILE = 'update.json';
const ARTIFACTS_DIR = path.resolve('artifacts');

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
  if (!fs.existsSync(directory)) {
    console.error(`[Error]: Directory "${directory}" does not exist.`);
    return { fileName: '', signaturePath: '' };
  }

  const files = fs.readdirSync(directory);
  let fileName = '';
  let signaturePath = '';

  files.forEach(file => {
    if (file.endsWith('.msi') || file.endsWith('.app.tar.gz') || file.endsWith('.AppImage')) {
      fileName = file;
    } else if (file.endsWith('.msi.sig') || file.endsWith('.app.tar.gz.sig') || file.endsWith('.AppImage.sig')) {
      signaturePath = path.join(directory, file);
    }
  });

  if (!fileName) {
    console.error(`[Error]: No artifact found in "${directory}".`);
  }

  return { fileName, signaturePath };
}

function readSignature(signaturePath) {
  return fs.existsSync(signaturePath) ? fs.readFileSync(signaturePath, 'utf8') : '';
}

// Resolve artifact paths and signatures
['windows-x86_64', 'darwin-aarch64', 'darwin-x86_64', 'linux-x86_64'].forEach(platformKey => {
  const platformDir = path.join(ARTIFACTS_DIR, platformKey);
  const { fileName, signaturePath } = getFileNameAndSignature(platformDir);
  
  console.log(`Processing platform: ${platformKey}`);
  console.log(`Found fileName: ${fileName}`);
  console.log(`Found signaturePath: ${signaturePath}`);

  if (fileName) {
    let urlFileName = fileName;
    if (platformKey === 'darwin-aarch64') {
      urlFileName = `aarch64-${fileName}`;
    }
    updateData.platforms[platformKey].url = `https://github.com/querent-ai/distribution/releases/download/${process.env.ASSET_VERSION}/${urlFileName}`;
    updateData.platforms[platformKey].signature = readSignature(signaturePath);
  }
});

console.log('updateData:', updateData);
// Remove platforms with no URLs
Object.entries(updateData.platforms).forEach(([key, value]) => {
  if (!value.url) {
    console.warn(`[Warning]: No URL for platform "${key}", removing from update.json.`);
    delete updateData.platforms[key];
  }
});

// Write update.json to the root directory
fs.writeFileSync(UPDATE_JSON_FILE, JSON.stringify(updateData, null, 2));

console.log('update.json generated successfully:', path.resolve(UPDATE_JSON_FILE));
