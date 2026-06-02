// place files you want to import through the `$lib` alias in this folder.

import { PUBLIC_API_BASE } from '$env/static/public';
import type { paths } from './api';
import createClient from 'openapi-fetch';

export const api = createClient<paths>({ baseUrl: PUBLIC_API_BASE });

export type ApiRes<P extends keyof paths> = paths[P] extends {
	get: {
		responses: {
			200: {
				content: {
					'application/json': infer R;
				};
			};
		};
	};
}
	? R
	: never;

export async function sign_out() {
	await api.POST('/session/sign-out');
}
