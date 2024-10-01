import { makeRequest } from "$lib/api";
import { fail, redirect } from "@sveltejs/kit";

export const actions = {
	create: async ({ cookies, request, fetch }) => {
		const data = await request.formData();

		let errors = [];

		let userName = data.get('user_name');
		let password = data.get('password');
		let confirmPassword = data.get('password_confirmation');

		if (!userName) {
			errors.push('Username is required.');
		}

		if (!password) {
			errors.push('Password is required.');
		}

		if (confirmPassword != password) {
			errors.push('Passwords do not match.');
		}

		if (errors.length > 0) {
			return fail(422, {
				errors
			});
		}

		let res = await makeRequest(fetch, 'api/users', {
			method: 'POST',
			body: JSON.stringify({
				password,
				user_email: userName
			})
		});

		if (res.status === 200) {
			throw redirect(
				301,
				'/login',
			)
		}
		else {
			let response = await res.json();
			errors.push({
				"UserExists": "This user is already registered. Perhaps you meant to log in?",
				"PasswordTooShort": "Your password is too short. It must be at least eight characters long.",
				"Other": "Unknown Error"
			}[response.message.error] || response.message.error);
			return fail(422, {
				errors
			});
		}
	}
	
	/*catch (error) {
		return fail(422, {
			description: data.get('description'),
			error: error.message
		});
	}*/
};