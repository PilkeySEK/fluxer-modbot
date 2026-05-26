import {
	FLUXER_API_BASE,
	OAUTH_CLIENT_ID,
	OAUTH_CLIENT_SECRET,
	OAUTH_REDIRECT_URI,
} from '$env/static/private';
import { saveSession } from '$lib/server/session';
import { error, redirect, type RequestHandler } from '@sveltejs/kit';

export const GET: RequestHandler = async ({ url, cookies }) => {
	const code = url.searchParams.get('code');
	const state = url.searchParams.get('state');
	const storedState = cookies.get('oauth_state');

	if (state == undefined || state !== storedState) {
		throw error(403, 'Invalid state parameter');
	}
	cookies.delete('oauth_state', { path: '/' });

	if (code === null) {
		throw error(400, 'Missing authorization code');
	}

	const tokenRes = await fetch(`${FLUXER_API_BASE}/oauth2/token`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/x-www-form-urlencoded',
		},
		body: new URLSearchParams({
			client_id: OAUTH_CLIENT_ID,
			client_secret: OAUTH_CLIENT_SECRET,
			grant_type: 'authorization_code',
			code,
			redirect_uri: OAUTH_REDIRECT_URI,
		}),
	});

	if (!tokenRes.ok) {
		throw error(500, 'Failed to exchange code for token');
	}

	const json: { access_token?: string } = await tokenRes.json();

	if (json.access_token === undefined) {
		throw error(500, 'Access token not present in Fluxer response');
	}

	const userRes = await fetch(`${FLUXER_API_BASE}/users/@me`, {
		headers: {
			Authorization: `Bearer ${json.access_token}`,
		},
	});
	if (!userRes.ok) {
		throw error(500, 'Failed to fetch user');
	}
	const user = await userRes.json();

	const sessionId = crypto.randomUUID();
	await saveSession(sessionId, user);

	cookies.set('session_id', sessionId, {
		path: '/',
		httpOnly: true,
		secure: true,
		sameSite: 'lax',
		maxAge: 60 * 60 * 24 * 7, // 7 days
	});

	throw redirect(302, '/dashboard');
};
