interface ImportMetaEnv {
	readonly VITE_DRIVE_CLIENT_ID: string;
	readonly VITE_DRIVE_REDIRECT_URL: string;
	readonly VITE_DRIVE_CLIENT_SECRET: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
