// See https://svelte.dev/docs/kit/types#app.d.ts
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
			} | null;
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
