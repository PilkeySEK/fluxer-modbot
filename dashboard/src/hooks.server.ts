import { cleanupSessions, getUserData } from '$lib/server/database';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
	cleanupSessions();

	const sessionId = event.cookies.get('session_id');

	if (sessionId !== undefined) {
		const user = await getUserData(sessionId);
		if (user === null) {
			event.locals.user = null;
		} else {
			event.locals.user = { ...user, server_data: { ...user.server_data, session_id: sessionId } };
		}
	} else {
		event.locals.user = null;
	}

	return resolve(event);
};
