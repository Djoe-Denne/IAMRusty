import React from 'react';
import { Link } from 'react-router-dom';
import { UserPlus, LogIn, Settings, CheckCircle, XCircle } from 'lucide-react';

export const HomePage: React.FC = () => {
  const testPages = [
    {
      title: 'Signup',
      description: 'Create a new user account with email and password',
      path: '/signup',
      icon: UserPlus,
      testId: 'page-link-signup'
    },
    {
      title: 'Login',
      description: 'Authenticate with existing user credentials',
      path: '/login',
      icon: LogIn,
      testId: 'page-link-login'
    },
    {
      title: 'Complete Registration',
      description: 'Set username after initial signup (requires token)',
      path: '/complete-registration',
      icon: Settings,
      testId: 'page-link-complete-registration'
    },
    {
      title: 'Success Page',
      description: 'Test success/OK page for positive outcomes',
      path: '/ok',
      icon: CheckCircle,
      testId: 'page-link-ok'
    },
    {
      title: 'Error Page',
      description: 'Test error/KO page for negative outcomes',
      path: '/ko',
      icon: XCircle,
      testId: 'page-link-ko'
    }
  ];

  return (
    <div className="max-w-4xl mx-auto">
      <div className="text-center mb-12">
        <h1 className="text-4xl font-bold text-gray-900 mb-4" data-testid="home-title">
          IAM Service Test Application
        </h1>
        <p className="text-xl text-gray-600 max-w-2xl mx-auto" data-testid="home-description">
          A simple testing interface for the IAM service API endpoints. Each page is designed with meaningful 
          selectors for Playwright testing.
        </p>
      </div>

      <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
        {testPages.map((page) => {
          const IconComponent = page.icon;
          return (
            <Link
              key={page.path}
              to={page.path}
              data-testid={page.testId}
              className="block p-6 bg-white rounded-lg shadow-sm border border-gray-200 hover:shadow-md hover:border-blue-300 transition-all duration-200 group"
            >
              <div className="flex items-center mb-4">
                <div className="p-3 bg-blue-100 rounded-lg group-hover:bg-blue-200 transition-colors">
                  <IconComponent className="w-6 h-6 text-blue-600" />
                </div>
                <h3 className="ml-4 text-lg font-semibold text-gray-900">{page.title}</h3>
              </div>
              <p className="text-gray-600 text-sm leading-relaxed">{page.description}</p>
            </Link>
          );
        })}
      </div>

      <div className="mt-12 bg-blue-50 rounded-lg p-6">
        <h2 className="text-lg font-semibold text-blue-900 mb-3">Testing Notes</h2>
        <ul className="text-blue-800 text-sm space-y-2">
          <li>• All form inputs have meaningful IDs and data-testid attributes</li>
          <li>• Error messages are displayed with consistent selectors</li>
          <li>• Navigation elements include test-friendly identifiers</li>
          <li>• API responses are handled with proper success/error routing</li>
        </ul>
      </div>
    </div>
  );
};