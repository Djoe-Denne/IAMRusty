import React, { useEffect, useState, useRef } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { Loader2, AlertCircle } from 'lucide-react';

export const OAuthCallbackPage: React.FC = () => {
  const navigate = useNavigate();
  const { provider } = useParams<{ provider: string }>();
  const [searchParams] = useSearchParams();
  const [error, setError] = useState('');
  const hasProcessed = useRef(false);

  useEffect(() => {
    const handleCallback = async () => {
      // Prevent duplicate calls (important for React StrictMode in development)
      if (hasProcessed.current) {
        console.log('OAuth callback already processed, skipping duplicate call');
        return;
      }
      hasProcessed.current = true;
      
      const code = searchParams.get('code');
      const state = searchParams.get('state');
      const errorParam = searchParams.get('error');

      console.log('Processing OAuth callback:', { provider, code: code?.substring(0, 10) + '...', state, errorParam });

      // Check for OAuth errors first
      if (errorParam) {
        navigate(`/ko?error=oauth_error&details=${encodeURIComponent(errorParam)}`);
        return;
      }

      // Check for required parameters
      if (!code || !provider) {
        navigate('/ko?error=missing_parameters');
        return;
      }

      try {
        // Call the API callback endpoint with the same parameters
        const callbackUrl = `/api/auth/${provider}/callback?code=${encodeURIComponent(code)}${state ? `&state=${encodeURIComponent(state)}` : ''}`;
        
        console.log('Calling API callback:', callbackUrl);
        
        const response = await fetch(callbackUrl, {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
          },
        });

        console.log('API callback response status:', response.status);
        const data = await response.json();
        console.log('API callback response data:', data);

        if (response.status === 200) {
          if (data.operation === 'login') {
            // User already has username - complete login
            localStorage.setItem('access_token', data.access_token);
            localStorage.setItem('refresh_token', data.refresh_token);
            navigate(`/ok?action=login&user=${encodeURIComponent(data.user.username)}`);
          } else if (data.operation === 'link') {
            // Provider linking successful
            navigate(`/ok?action=link&provider=${encodeURIComponent(provider || '')}`);
          } else {
            // Unexpected operation type
            navigate('/ko?error=unknown_operation');
          }
        } else if (response.status === 202) {
          // Registration required - redirect to complete registration with token
          console.log('Registration required, navigating to complete registration');
          const params = new URLSearchParams({
            token: data.registration_token,
            email: data.provider_info?.email || '',
            suggested_username: data.provider_info?.suggested_username || ''
          });
          navigate(`/complete-registration?${params.toString()}`);
        } else if (response.status === 409) {
          // Provider linking conflict
          navigate(`/ko?error=provider_conflict&details=${encodeURIComponent(data.message || 'Provider linking conflict')}`);
        } else {
          // Other errors
          const errorMessage = data.message || data.error || 'OAuth authentication failed';
          navigate(`/ko?error=oauth_failed&details=${encodeURIComponent(errorMessage)}`);
        }
      } catch (error) {
        console.error('OAuth callback error:', error);
        setError('Network error occurred during authentication');
        setTimeout(() => {
          navigate('/ko?error=network_error');
        }, 2000);
      }
    };

    handleCallback();
  }, [navigate, provider, searchParams]);

  if (error) {
    return (
      <div className="max-w-md mx-auto">
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
          <div className="text-center">
            <AlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <h1 className="text-xl font-semibold text-gray-900 mb-2">Authentication Error</h1>
            <p className="text-gray-600 mb-4">{error}</p>
            <button
              onClick={() => navigate('/')}
              className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 transition-colors"
            >
              Go Home
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-md mx-auto">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <div className="text-center">
          <Loader2 className="w-12 h-12 text-blue-500 mx-auto mb-4 animate-spin" />
          <h1 className="text-xl font-semibold text-gray-900 mb-2">
            Completing Authentication
          </h1>
          <p className="text-gray-600">
            Please wait while we process your {provider} authentication...
          </p>
        </div>
      </div>
    </div>
  );
}; 