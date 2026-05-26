import { OAUTH_BASE, OAUTH_CLIENT_ID, OAUTH_REDIRECT_URI, OAUTH_SCOPE } from '$env/static/private';
import { redirect, type RequestHandler } from '@sveltejs/kit';

export const GET: RequestHandler = async ({ cookies }) => {
	const state = crypto.randomUUID();
	cookies.set('oauth_state', state, {
		path: '/',
		httpOnly: true,
		secure: true,
		sameSite: 'lax',
		maxAge: 60 * 10,
	});

	const params = new URLSearchParams({
		client_id: OAUTH_CLIENT_ID,
		redirect_uri: OAUTH_REDIRECT_URI,
		response_type: 'code',
		scope: OAUTH_SCOPE,
		state,
	});

	throw redirect(302, `${OAUTH_BASE}?${params}`);
};
