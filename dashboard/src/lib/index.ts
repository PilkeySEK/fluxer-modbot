// place files you want to import through the `$lib` alias in this folder.

import type { paths } from './api';
import createClient from 'openapi-fetch';

/*export async function api_request<T>(path: string | URL | Request, init?: RequestInit): Promise<T> {
	const res = await fetch(path, init);

	const json: T | ApiErrorBody = await res.json();

	if (!res.ok) {
		throw new Error(`Api error ${res.status}: ${(json as ApiErrorBody).error}`);
	}

	return json as T;
}*/

export const api = createClient<paths>({ baseUrl: '/api/' });

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
