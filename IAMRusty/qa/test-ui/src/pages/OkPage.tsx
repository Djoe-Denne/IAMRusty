import React from 'react';
import { useSearchParams, Link } from 'react-router-dom';
import { CheckCircle, Home, User } from 'lucide-react';

export const OkPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const action = searchParams.get('action');
  const user = searchParams.get('user');

  const getSuccessMessage = () => {
    switch (action) {
      case 'login':
        return {
          title: 'Login Successful',
          message: `Welcome back${user ? `, ${user}` : ''}! You have successfully signed in.`,
          icon: CheckCircle
        };
      case 'registration':
        return {
          title: 'Registration Complete',
          message: `Welcome${user ? `, ${user}` : ''}! Your account has been created successfully.`,
          icon: User
        };
      case 'signup':
        return {
          title: 'Signup Successful',
          message: 'Your account has been created. Please complete your registration.',
          icon: CheckCircle
        };
      default:
        return {
          title: 'Success',
          message: 'Operation completed successfully.',
          icon: CheckCircle
        };
    }
  };

  const success = getSuccessMessage();
  const IconComponent = success.icon;

  return (
    <div className="max-w-md mx-auto">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
        <div className="mb-6">
          <div className="mx-auto w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mb-4">
            <IconComponent className="w-8 h-8 text-green-600" data-testid="ok-icon" />
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2" data-testid="ok-title">
            {success.title}
          </h1>
          <p className="text-gray-600" data-testid="ok-message">
            {success.message}
          </p>
        </div>

        {user && (
          <div className="mb-6 p-4 bg-green-50 border border-green-200 rounded-md">
            <p className="text-sm text-green-800" data-testid="ok-user-info">
              <strong>User:</strong> {user}
            </p>
            {action && (
              <p className="text-sm text-green-800 mt-1" data-testid="ok-action-info">
                <strong>Action:</strong> {action}
              </p>
            )}
          </div>
        )}

        <div className="flex flex-col space-y-3">
          <Link
            to="/"
            data-testid="ok-home-button"
            className="inline-flex items-center justify-center px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
          >
            <Home className="w-4 h-4 mr-2" />
            Back to Home
          </Link>
          
          <div className="flex space-x-3">
            <Link
              to="/login"
              data-testid="ok-login-button"
              className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 transition-colors text-center"
            >
              Login
            </Link>
            <Link
              to="/signup"
              data-testid="ok-signup-button"
              className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 transition-colors text-center"
            >
              Signup
            </Link>
          </div>
        </div>

        <div className="mt-6 pt-6 border-t border-gray-200">
          <p className="text-xs text-gray-500" data-testid="ok-timestamp">
            Success at {new Date().toLocaleString()}
          </p>
        </div>
      </div>
    </div>
  );
};