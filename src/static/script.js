document.addEventListener('DOMContentLoaded', () => {
    const loginView = document.getElementById('login-view');
    const registerView = document.getElementById('register-view');
    const showRegisterLink = document.getElementById('show-register');
    const showLoginLink = document.getElementById('show-login');

    const loginForm = document.getElementById('login-form');
    const registerForm = document.getElementById('register-form');
    const messageContainer = document.createElement('div');
    messageContainer.className = 'message-container';
    document.querySelector('.container').appendChild(messageContainer);

    const steps = Array.from(registerForm.querySelectorAll('.form-step'));
    const nextButtons = registerForm.querySelectorAll('.next-step');
    const prevButtons = registerForm.querySelectorAll('.prev-step');
    let currentStep = 0;

    // API endpoints - Use correct paths with /api/ prefix
    const API_ENDPOINTS = {
        register: '/api/register',
        login: '/api/login',  
        deviceLookup: '/api/device'
    };
    
    console.log('API endpoints configured:', API_ENDPOINTS);

    // Show feedback messages to user
    function showMessage(message, type = 'info') {
        messageContainer.innerHTML = `<div class="message ${type}">${message}</div>`;
        
        // Auto-hide after 5 seconds
        setTimeout(() => {
            messageContainer.innerHTML = '';
        }, 5000);
    }

    // Show specific step in registration form
    function showStep(stepIndex) {
        steps.forEach((step, index) => {
            step.style.display = index === stepIndex ? 'block' : 'none';
        });
        currentStep = stepIndex;
    }

    // Toggle between login and register views
    showRegisterLink.addEventListener('click', (e) => {
        e.preventDefault();
        loginView.style.display = 'none';
        registerView.style.display = 'block';
        showStep(0); // Show first step of registration
    });

    showLoginLink.addEventListener('click', (e) => {
        e.preventDefault();
        registerView.style.display = 'none';
        loginView.style.display = 'block';
    });

    // Handle navigation between registration steps
    nextButtons.forEach(button => {
        button.addEventListener('click', () => {
            // Basic validation: check if required fields in current step are filled
            const currentStepFields = steps[currentStep].querySelectorAll('input[required]');
            let allFilled = true;
            currentStepFields.forEach(field => {
                if (!field.value.trim()) {
                    allFilled = false;
                    field.style.borderColor = 'red'; // Highlight empty required fields
                } else {
                    field.style.borderColor = '#ddd';
                }
            });

            if (allFilled && currentStep < steps.length - 1) {
                showStep(currentStep + 1);
            } else if (!allFilled) {
                showMessage('Please fill in all required fields for this step.', 'error');
            }
        });
    });

    prevButtons.forEach(button => {
        button.addEventListener('click', () => {
            if (currentStep > 0) {
                showStep(currentStep - 1);
            }
        });
    });

    // Login form submission
    loginForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const formData = new FormData(loginForm);
        const data = {
            email: formData.get('email'),
            password: formData.get('password')
        };
        
        try {
            const loginButton = loginForm.querySelector('button[type="submit"]');
            loginButton.disabled = true;
            loginButton.textContent = 'Logging in...';
            
            console.log('Sending login request to:', API_ENDPOINTS.login);
            console.log('With data:', JSON.stringify(data));
            
            const response = await fetch(API_ENDPOINTS.login, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(data)
            });
            
            console.log('Login response status:', response.status);
            
            const result = await response.json();
            console.log('Login response data:', result);
            
            if (!response.ok) {
                showMessage(result.message || 'Login failed. Please check your credentials.', 'error');
            } else {
                showMessage('Login successful!', 'success');
                
                // Store token and user info in localStorage
                localStorage.setItem('auth_token', result.token);
                if (result.user_id) {
                    localStorage.setItem('user_id', result.user_id);
                }
                
                // Redirect or update UI for logged-in state
                // window.location.href = '/dashboard'; // Uncomment if redirecting
            }
        } catch (error) {
            console.error('Error during login:', error);
            showMessage('Network error. Please try again later.', 'error');
        } finally {
            const loginButton = loginForm.querySelector('button[type="submit"]');
            loginButton.disabled = false;
            loginButton.textContent = 'Login';
        }
    });

    // Registration form submission
    registerForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        
        // Final validation for the last step
        const lastStepFields = steps[currentStep].querySelectorAll('input[required]');
        let allFilled = true;
        lastStepFields.forEach(field => {
            if (!field.value.trim()) {
                allFilled = false;
                field.style.borderColor = 'red';
            } else {
                field.style.borderColor = '#ddd';
            }
        });

        if (!allFilled) {
            showMessage('Please fill in all required fields for this step.', 'error');
            return;
        }

        // Gather all form data
        const formData = new FormData(registerForm);
        const data = {
            serial_number: formData.get('serial_number'),
            email: formData.get('email'),
            username: formData.get('username'),
            password: formData.get('password'),
            device_name: formData.get('device_name')
        };

        // Debug logs
        console.log('Sending registration data:', data);
        console.log('To endpoint:', API_ENDPOINTS.register);
        
        try {
            const registerButton = registerForm.querySelector('button[type="submit"]');
            registerButton.disabled = true;
            registerButton.textContent = 'Creating account...';
            
            const response = await fetch(API_ENDPOINTS.register, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(data)
            });
            
            console.log('Registration response status:', response.status);
            
            const result = await response.json();
            console.log('Registration response data:', result);
            
            if (!response.ok) {
                showMessage(result.message || 'Registration failed. Please try again.', 'error');
            } else {
                showMessage('Registration successful! You can now log in.', 'success');
                
                // Reset form and return to login view
                registerForm.reset();
                showStep(0);
                document.getElementById('register-view').style.display = 'none';
                document.getElementById('login-view').style.display = 'block';
            }
        } catch (error) {
            console.error('Error during registration:', error);
            showMessage('Network error. Please try again later.', 'error');
        } finally {
            const registerButton = registerForm.querySelector('button[type="submit"]');
            registerButton.disabled = false;
            registerButton.textContent = 'Register';
        }
    });

    // Function to check if user is already logged in
    function checkLoggedInStatus() {
        const token = localStorage.getItem('auth_token');
        if (token) {
            // You could verify the token validity here with a dedicated endpoint
            // For now, we'll just assume it's valid if it exists
            showMessage('You are already logged in.', 'info');
            // Optionally redirect to dashboard or show logged-in UI
        }
    }

    // Add a logout function
    window.logout = function() {
        localStorage.removeItem('auth_token');
        localStorage.removeItem('user_id');
        showMessage('You have been logged out.', 'info');
        // Refresh page or update UI for logged-out state
    };

    // Function to check device's serial number before registration
    async function checkSerialNumber(serialNumber) {
        try {
            const response = await fetch(API_ENDPOINTS.deviceLookup, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ serial_number: serialNumber })
            });
            
            return await response.json();
        } catch (error) {
            console.error('Error checking serial number:', error);
            return null;
        }
    }

    // Add serial number validation on first step
    const serialInput = document.getElementById('serial-number');
    const serialNextButton = steps[0].querySelector('.next-step');
    
    serialInput.addEventListener('change', async () => {
        const serialNumber = serialInput.value.trim();
        if (serialNumber) {
            serialNextButton.disabled = true;
            serialNextButton.textContent = 'Checking...';
            
            try {
                const result = await checkSerialNumber(serialNumber);
                if (result && result.device_owner) {
                    // Serial number is already registered
                    showMessage('This device is already registered.', 'error');
                    serialInput.style.borderColor = 'red';
                } else {
                    serialInput.style.borderColor = '#ddd';
                }
            } catch (error) {
                console.error('Error during serial check:', error);
            } finally {
                serialNextButton.disabled = false;
                serialNextButton.textContent = 'Next';
            }
        }
    });

    // Check logged-in status on page load
    checkLoggedInStatus();
    
    // Initialize with the first step of registration visible if register view is active
    if (registerView.style.display === 'block') {
        showStep(0);
    }
});