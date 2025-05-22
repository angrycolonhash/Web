document.addEventListener('DOMContentLoaded', () => {
    const loginView = document.getElementById('login-view');
    const registerView = document.getElementById('register-view');
    const showRegisterLink = document.getElementById('show-register');
    const showLoginLink = document.getElementById('show-login');

    const loginForm = document.getElementById('login-form');
    const registerForm = document.getElementById('register-form');

    const steps = Array.from(registerForm.querySelectorAll('.form-step'));
    const nextButtons = registerForm.querySelectorAll('.next-step');
    const prevButtons = registerForm.querySelectorAll('.prev-step');
    let currentStep = 0;

    function showStep(stepIndex) {
        steps.forEach((step, index) => {
            step.style.display = index === stepIndex ? 'block' : 'none';
        });
        currentStep = stepIndex;
    }

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
                alert('Please fill in all required fields for this step.');
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

    loginForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const formData = new FormData(loginForm);
        const data = Object.fromEntries(formData.entries());
        console.log('Login data:', data);
        // Here you would typically send 'data' to your backend API
        alert('Login submitted! Check console for data.');
        loginForm.reset();
    });

    registerForm.addEventListener('submit', (e) => {
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
            alert('Please fill in all required fields for this step.');
            return;
        }

        const formData = new FormData(registerForm);
        const data = Object.fromEntries(formData.entries());
        console.log('Register data:', data);
        // Here you would typically send 'data' to your backend API
        // The 'account_created_at' field would be set by the backend
        alert('Registration submitted! Check console for data.');
        registerForm.reset();
        showStep(0); // Reset to first step
        // Optionally switch back to login view
        // registerView.style.display = 'none';
        // loginView.style.display = 'block';
    });

    // Initialize with the first step of registration visible if register view is active
    if (registerView.style.display === 'block') {
        showStep(0);
    }
});