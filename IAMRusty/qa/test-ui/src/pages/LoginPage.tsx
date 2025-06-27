import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Mail, Lock, AlertCircle, Loader2, Github } from 'lucide-react';

export const LoginPage: React.FC = () => {
  const navigate = useNavigate();
  const [formData, setFormData] = useState({
    email: '',
    password: ''
  });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [apiError, setApiError] = useState('');

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setFormData(prev => ({ ...prev, [name]: value }));
    // Clear error when user starts typing
    if (errors[name]) {
      setErrors(prev => ({ ...prev, [name]: '' }));
    }
    if (apiError) setApiError('');
  };

  const validateForm = () => {
    const newErrors: Record<string, string> = {};

    if (!formData.email) {
      newErrors.email = 'Email is required';
    } else if (!/\S+@\S+\.\S+/.test(formData.email)) {
      newErrors.email = 'Email format is invalid';
    }

    if (!formData.password) {
      newErrors.password = 'Password is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateForm()) return;

    setIsLoading(true);
    setApiError('');

    try {
      const response = await fetch('/api/auth/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(formData),
      });

      const data = await response.json();

      if (response.ok) {
        // Success - store token and navigate to success page
        localStorage.setItem('access_token', data.access_token);
        localStorage.setItem('refresh_token', data.refresh_token);
        navigate('/ok?action=login&user=' + encodeURIComponent(data.user.username));
      } else {
        // Handle specific error cases
        if (response.status === 401) {
          setApiError('Invalid email or password');
        } else if (response.status === 423) {
          // Registration incomplete - redirect to complete registration
          const params = new URLSearchParams({
            token: data.registration_token,
            email: formData.email
          });
          navigate(`/complete-registration?${params.toString()}`);
        } else if (response.status === 422) {
          setApiError(data.message || 'Please check your input and try again');
        } else {
          setApiError(data.message || 'Login failed. Please try again.');
        }
      }
    } catch (error) {
      setApiError('Network error. Please check your connection and try again.');
    } finally {
      setIsLoading(false);
    }
  };

  const handleOAuthLogin = (provider: 'github' | 'gitlab') => {
    // Redirect to OAuth provider login endpoint
    window.location.href = `/api/auth/${provider}/login`;
  };

  const GitLabIcon = () => (
    <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 21.42l3.684-11.333H8.316L12 21.42z"/>
      <path d="M12 21.42l-3.684-11.333H1.684L12 21.42z"/>
      <path d="M1.684 10.087L.42 13.84a.86.86 0 00.32.99L12 21.42 1.684 10.087z"/>
      <path d="M1.684 10.087h6.632L6.947 1.82a.43.43 0 00-.816 0L1.684 10.087z"/>
      <path d="M12 21.42l3.684-11.333h6.632L12 21.42z"/>
      <path d="M22.316 10.087L23.58 13.84a.86.86 0 01-.32.99L12 21.42l10.316-11.333z"/>
      <path d="M22.316 10.087h-6.632L17.053 1.82a.43.43 0 01.816 0l4.447 8.267z"/>
    </svg>
  );

  return (
    <div className="max-w-md mx-auto">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="text-center mb-8">
          <h1 className="text-2xl font-bold text-gray-900" data-testid="login-title">
            Sign In
          </h1>
          <p className="text-gray-600 mt-2">Access your account</p>
        </div>

        {/* OAuth Provider Buttons */}
        <div className="space-y-3 mb-6">
          <button
            onClick={() => handleOAuthLogin('github')}
            data-testid="login-github-button"
            className="w-full flex items-center justify-center px-4 py-3 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors"
          >
            <Github className="w-5 h-5 mr-3" />
            Continue with GitHub
          </button>
          
          <button
            onClick={() => handleOAuthLogin('gitlab')}
            data-testid="login-gitlab-button"
            className="w-full flex items-center justify-center px-4 py-3 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-white bg-orange-600 hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-orange-500 transition-colors"
          >
            <GitLabIcon />
            <span className="ml-3">Continue with GitLab</span>
          </button>
        </div>

        {/* Divider */}
        <div className="relative mb-6">
          <div className="absolute inset-0 flex items-center">
            <div className="w-full border-t border-gray-300" />
          </div>
          <div className="relative flex justify-center text-sm">
            <span className="px-2 bg-white text-gray-500">Or continue with email</span>
          </div>
        </div>

        {apiError && (
          <div className="mb-6 p-3 bg-red-50 border border-red-200 rounded-md" data-testid="login-error">
            <div className="flex items-center">
              <AlertCircle className="w-4 h-4 text-red-500 mr-2" />
              <span className="text-red-700 text-sm">{apiError}</span>
            </div>
          </div>
        )}

        <form onSubmit={handleSubmit} data-testid="login-form">
          <div className="space-y-6">
            <div>
              <label htmlFor="login-email" className="block text-sm font-medium text-gray-700 mb-2">
                Email Address
              </label>
              <div className="relative">
                <Mail className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-5 h-5" />
                <input
                  type="email"
                  id="login-email"
                  name="email"
                  data-testid="login-email-input"
                  className={`w-full pl-10 pr-4 py-3 border rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-colors ${
                    errors.email ? 'border-red-300 bg-red-50' : 'border-gray-300'
                  }`}
                  placeholder="Enter your email"
                  value={formData.email}
                  onChange={handleInputChange}
                  disabled={isLoading}
                />
              </div>
              {errors.email && (
                <p className="mt-1 text-sm text-red-600" data-testid="login-email-error">
                  {errors.email}
                </p>
              )}
            </div>

            <div>
              <label htmlFor="login-password" className="block text-sm font-medium text-gray-700 mb-2">
                Password
              </label>
              <div className="relative">
                <Lock className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-5 h-5" />
                <input
                  type="password"
                  id="login-password"
                  name="password"
                  data-testid="login-password-input"
                  className={`w-full pl-10 pr-4 py-3 border rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-colors ${
                    errors.password ? 'border-red-300 bg-red-50' : 'border-gray-300'
                  }`}
                  placeholder="Enter your password"
                  value={formData.password}
                  onChange={handleInputChange}
                  disabled={isLoading}
                />
              </div>
              {errors.password && (
                <p className="mt-1 text-sm text-red-600" data-testid="login-password-error">
                  {errors.password}
                </p>
              )}
            </div>

            <button
              type="submit"
              data-testid="login-submit-button"
              disabled={isLoading}
              className="w-full bg-blue-600 text-white py-3 px-4 rounded-md hover:bg-blue-700 focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center"
            >
              {isLoading ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Signing In...
                </>
              ) : (
                'Sign In'
              )}
            </button>
          </div>
        </form>

        <div className="mt-6 text-center">
          <p className="text-sm text-gray-600">
            Don't have an account?{' '}
            <button
              onClick={() => navigate('/signup')}
              data-testid="login-signup-link"
              className="text-blue-600 hover:text-blue-700 font-medium"
            >
              Create one
            </button>
          </p>
        </div>
      </div>
    </div>
  );
};