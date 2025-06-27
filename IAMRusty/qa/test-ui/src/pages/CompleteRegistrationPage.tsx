import React, { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { User, AlertCircle, Loader2, CheckCircle } from 'lucide-react';

export const CompleteRegistrationPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [username, setUsername] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [apiError, setApiError] = useState('');
  const [isCheckingUsername, setIsCheckingUsername] = useState(false);
  const [usernameAvailable, setUsernameAvailable] = useState(false);
  const [suggestions, setSuggestions] = useState<string[]>([]);

  const registrationToken = searchParams.get('token');
  const email = searchParams.get('email');
  const suggestedUsername = searchParams.get('suggested_username');

  useEffect(() => {
    if (!registrationToken) {
      navigate('/ko?error=missing_token');
    }
    
    // Set suggested username if provided
    if (suggestedUsername && !username) {
      setUsername(suggestedUsername);
      checkUsernameAvailability(suggestedUsername);
    }
  }, [registrationToken, navigate, suggestedUsername]);

  const checkUsernameAvailability = async (usernameToCheck: string) => {
    if (usernameToCheck.length < 3) {
      setUsernameAvailable(false);
      setSuggestions([]);
      return;
    }

    setIsCheckingUsername(true);
    try {
      const response = await fetch(`/api/auth/username/check?username=${encodeURIComponent(usernameToCheck)}`);
      const data = await response.json();
      
      if (response.ok) {
        setUsernameAvailable(data.available);
        setSuggestions(data.suggestions || []);
      }
    } catch (error) {
      console.error('Username check failed:', error);
    } finally {
      setIsCheckingUsername(false);
    }
  };

  const handleUsernameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setUsername(value);
    
    // Clear errors when user starts typing
    if (errors.username) {
      setErrors(prev => ({ ...prev, username: '' }));
    }
    if (apiError) setApiError('');

    // Debounce username availability check
    const timeoutId = setTimeout(() => {
      checkUsernameAvailability(value);
    }, 300);

    return () => clearTimeout(timeoutId);
  };

  const validateForm = () => {
    const newErrors: Record<string, string> = {};

    if (!username) {
      newErrors.username = 'Username is required';
    } else if (username.length < 3) {
      newErrors.username = 'Username must be at least 3 characters';
    } else if (username.length > 50) {
      newErrors.username = 'Username must be less than 50 characters';
    } else if (!/^[a-zA-Z0-9_-]+$/.test(username)) {
      newErrors.username = 'Username can only contain letters, numbers, underscores, and hyphens';
    } else if (!usernameAvailable) {
      newErrors.username = 'Username is not available';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateForm() || !registrationToken) return;

    setIsLoading(true);
    setApiError('');

    try {
      const response = await fetch('/api/auth/complete-registration', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          registration_token: registrationToken,
          username
        }),
      });

      const data = await response.json();

      if (response.ok) {
        // Success - store tokens and navigate to success page
        localStorage.setItem('access_token', data.access_token);
        localStorage.setItem('refresh_token', data.refresh_token);
        navigate('/ok?action=registration&user=' + encodeURIComponent(data.user.username));
      } else {
        // Handle specific error cases
        if (response.status === 400) {
          setApiError('Registration token is invalid or expired');
        } else if (response.status === 409) {
          setApiError('Username is already taken');
        } else if (response.status === 422) {
          setApiError(data.message || 'Please check your input and try again');
        } else {
          setApiError(data.message || 'Registration failed. Please try again.');
        }
      }
    } catch (error) {
      setApiError('Network error. Please check your connection and try again.');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSuggestionClick = (suggestion: string) => {
    setUsername(suggestion);
    checkUsernameAvailability(suggestion);
  };

  return (
    <div className="max-w-md mx-auto">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="text-center mb-8">
          <h1 className="text-2xl font-bold text-gray-900" data-testid="complete-registration-title">
            Complete Registration
          </h1>
          <p className="text-gray-600 mt-2">
            {email && <span className="block text-sm">Welcome! {email}</span>}
            Choose your username to finish setting up your account
          </p>
        </div>

        {apiError && (
          <div className="mb-6 p-3 bg-red-50 border border-red-200 rounded-md" data-testid="complete-registration-error">
            <div className="flex items-center">
              <AlertCircle className="w-4 h-4 text-red-500 mr-2" />
              <span className="text-red-700 text-sm">{apiError}</span>
            </div>
          </div>
        )}

        <form onSubmit={handleSubmit} data-testid="complete-registration-form">
          <div className="space-y-6">
            <div>
              <label htmlFor="registration-username" className="block text-sm font-medium text-gray-700 mb-2">
                Username
              </label>
              <div className="relative">
                <User className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-5 h-5" />
                <input
                  type="text"
                  id="registration-username"
                  name="username"
                  data-testid="registration-username-input"
                  className={`w-full pl-10 pr-10 py-3 border rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-colors ${
                    errors.username ? 'border-red-300 bg-red-50' : 
                    usernameAvailable && username.length >= 3 ? 'border-green-300 bg-green-50' : 
                    'border-gray-300'
                  }`}
                  placeholder="Enter your username"
                  value={username}
                  onChange={handleUsernameChange}
                  disabled={isLoading}
                />
                <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
                  {isCheckingUsername ? (
                    <Loader2 className="w-4 h-4 text-gray-400 animate-spin" />
                  ) : usernameAvailable && username.length >= 3 ? (
                    <CheckCircle className="w-4 h-4 text-green-500" />
                  ) : null}
                </div>
              </div>
              {errors.username && (
                <p className="mt-1 text-sm text-red-600" data-testid="registration-username-error">
                  {errors.username}
                </p>
              )}
              {usernameAvailable && username.length >= 3 && (
                <p className="mt-1 text-sm text-green-600" data-testid="registration-username-available">
                  Username is available
                </p>
              )}
              <p className="mt-1 text-xs text-gray-500">
                3-50 characters, letters, numbers, underscores, and hyphens only
              </p>

              {suggestions.length > 0 && (
                <div className="mt-3" data-testid="username-suggestions">
                  <p className="text-sm text-gray-600 mb-2">Suggestions:</p>
                  <div className="flex flex-wrap gap-2">
                    {suggestions.map((suggestion, index) => (
                      <button
                        key={index}
                        type="button"
                        onClick={() => handleSuggestionClick(suggestion)}
                        data-testid={`username-suggestion-${index}`}
                        className="px-3 py-1 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 transition-colors"
                      >
                        {suggestion}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>

            <button
              type="submit"
              data-testid="registration-submit-button"
              disabled={isLoading || !usernameAvailable || username.length < 3}
              className="w-full bg-blue-600 text-white py-3 px-4 rounded-md hover:bg-blue-700 focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center"
            >
              {isLoading ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Completing Registration...
                </>
              ) : (
                'Complete Registration'
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};