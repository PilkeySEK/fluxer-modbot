import {
	FLUXER_API_BASE,
	OAUTH_CLIENT_ID,
	OAUTH_CLIENT_SECRET,
	OAUTH_REDIRECT_URI,
	OAUTH_SCOPE,
} from '$env/static/private';
import {
	OauthAccessTokenResponseSchema,
	saveSession,
	UserFluxerApiSchema,
} from '$lib/server/session';
import { error, redirect, type RequestHandler } from '@sveltejs/kit';

export const GET: RequestHandler = async ({ url, cookies }) => {
	const code = url.searchParams.get('code');
	const state = url.searchParams.get('state');
	const storedState = cookies.get('oauth_state');

	if (state === undefined || state !== storedState) {
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

	const access_token_response_json = await tokenRes.json();

	const result = OauthAccessTokenResponseSchema.safeParse(access_token_response_json);

	if (!result.success) {
		throw error(500, 'Fluxer API returned invalid OAuth2 token response');
	}

	const access_token_response = result.data;

	if (
		access_token_response.scope !== OAUTH_SCOPE &&
		access_token_response.scope !== OAUTH_SCOPE.replace(' ', '+')
	) {
		throw error(500, 'Invalid scope, expected ');
	}

	const userRes = await fetch(`${FLUXER_API_BASE}/users/@me`, {
		headers: {
			Authorization: `Bearer ${access_token_response.access_token}`,
		},
	});
	if (!userRes.ok) {
		throw error(500, 'Failed to fetch user');
	}
	const user_zod_result = UserFluxerApiSchema.safeParse(await userRes.json());

	if (!user_zod_result.success) {
		throw error(500, 'Fluxer responded with an invalid user schema');
	}

	const user = user_zod_result.data;

	// TODO: Store is_bot_admin in the API properly?
	const db_user = { ...user, is_bot_admin: false, server_data: access_token_response };

	const sessionId = crypto.randomUUID();
	// Expire sessions after 24h
	await saveSession(sessionId, new Date(Date.now() + 1000 * 60 * 60 * 24), db_user);

	cookies.set('session_id', sessionId, {
		path: '/',
		httpOnly: true,
		secure: true,
		sameSite: 'lax',
		maxAge: 60 * 60 * 24 * 7, // 7 days
	});

	throw redirect(302, '/dashboard');
};
