import React from 'react';
import { useSearchParams, Link } from 'react-router-dom';
import { XCircle, Home, RefreshCw, AlertTriangle } from 'lucide-react';

export const KoPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const error = searchParams.get('error');
  const message = searchParams.get('message');
  const action = searchParams.get('action');

  const getErrorMessage = () => {
    switch (error) {
      case 'missing_token':
        return {
          title: 'Missing Registration Token',
          message: 'Registration token is required to complete your account setup.',
          icon: AlertTriangle
        };
      case 'invalid_credentials':
        return {
          title: 'Invalid Credentials',
          message: 'The email or password you entered is incorrect.',
          icon: XCircle
        };
      case 'registration_incomplete':
        return {
          title: 'Registration Incomplete',
          message: 'Your account registration is not complete. Please finish the registration process.',
          icon: AlertTriangle
        };
      case 'network_error':
        return {
          title: 'Network Error',
          message: 'Unable to connect to the server. Please check your internet connection.',
          icon: XCircle
        };
      case 'validation_error':
        return {
          title: 'Validation Error',
          message: message || 'Please check your input and try again.',
          icon: AlertTriangle
        };
      case 'oauth_error':
        return {
          title: 'OAuth Authentication Error',
          message: searchParams.get('details') || 'OAuth authentication failed.',
          icon: XCircle
        };
      case 'oauth_failed':
        return {
          title: 'OAuth Authentication Failed',
          message: searchParams.get('details') || 'Authentication with the provider failed.',
          icon: XCircle
        };
      case 'provider_conflict':
        return {
          title: 'Provider Already Linked',
          message: searchParams.get('details') || 'This provider is already linked to another account.',
          icon: AlertTriangle
        };
      case 'missing_parameters':
        return {
          title: 'Missing OAuth Parameters',
          message: 'Required OAuth parameters are missing. Please try the authentication flow again.',
          icon: AlertTriangle
        };
      case 'unknown_operation':
        return {
          title: 'Unknown Operation',
          message: 'The OAuth response contains an unknown operation type.',
          icon: AlertTriangle
        };
      default:
        return {
          title: 'Error',
          message: message || 'An unexpected error occurred. Please try again.',
          icon: XCircle
        };
    }
  };

  const errorInfo = getErrorMessage();
  const IconComponent = errorInfo.icon;

  return (
    <div className="max-w-md mx-auto">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
        <div className="mb-6">
          <div className="mx-auto w-16 h-16 bg-red-100 rounded-full flex items-center justify-center mb-4">
            <IconComponent className="w-8 h-8 text-red-600" data-testid="ko-icon" />
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2" data-testid="ko-title">
            {errorInfo.title}
          </h1>
          <p className="text-gray-600" data-testid="ko-message">
            {errorInfo.message}
          </p>
        </div>

        {(error || action) && (
          <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-md">
            {error && (
              <p className="text-sm text-red-800" data-testid="ko-error-code">
                <strong>Error Code:</strong> {error}
              </p>
            )}
            {action && (
              <p className="text-sm text-red-800 mt-1" data-testid="ko-action-info">
                <strong>Failed Action:</strong> {action}
              </p>
            )}
          </div>
        )}

        <div className="flex flex-col space-y-3">
          <button
            onClick={() => window.location.reload()}
            data-testid="ko-retry-button"
            className="inline-flex items-center justify-center px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 transition-colors"
          >
            <RefreshCw className="w-4 h-4 mr-2" />
            Try Again
          </button>
          
          <Link
            to="/"
            data-testid="ko-home-button"
            className="inline-flex items-center justify-center px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 transition-colors"
          >
            <Home className="w-4 h-4 mr-2" />
            Back to Home
          </Link>
          
          <div className="flex space-x-3">
            <Link
              to="/login"
              data-testid="ko-login-button"
              className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 transition-colors text-center"
            >
              Login
            </Link>
            <Link
              to="/signup"
              data-testid="ko-signup-button"
              className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 transition-colors text-center"
            >
              Signup
            </Link>
          </div>
        </div>

        <div className="mt-6 pt-6 border-t border-gray-200">
          <p className="text-xs text-gray-500" data-testid="ko-timestamp">
            Error occurred at {new Date().toLocaleString()}
          </p>
        </div>
      </div>
    </div>
  );
};