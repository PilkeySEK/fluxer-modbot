// See https://svelte.dev/docs/kit/types#app.d.ts

import { OAuthAccessTokenResponse } from '$lib/server/session';

// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			user: {
				id: string;
				username: string;
				discriminator: string;
				global_name: string | null;
				avatar: string | null;
				avatar_color: number | null;
				flags: number;
				// is_staff: boolean,
				email: string | null;
				bio: string | null;
				pronouns: string | null;
				is_bot_admin: boolean;
				server_data: OAuthAccessTokenResponse & { session_id: string };
			} | null;
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
