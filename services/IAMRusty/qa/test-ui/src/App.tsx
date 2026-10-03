import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Header } from './components/Header';
import { SignupPage } from './pages/SignupPage';
import { LoginPage } from './pages/LoginPage';
import { CompleteRegistrationPage } from './pages/CompleteRegistrationPage';
import { OAuthCallbackPage } from './pages/OAuthCallbackPage';
import { OkPage } from './pages/OkPage';
import { KoPage } from './pages/KoPage';
import { HomePage } from './pages/HomePage';

function App() {
  return (
    <Router>
      <div className="min-h-screen bg-gray-50">
        <Header />
        <main className="container mx-auto px-4 py-8">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/signup" element={<SignupPage />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/complete-registration" element={<CompleteRegistrationPage />} />
            <Route path="/oauth/:provider/callback" element={<OAuthCallbackPage />} />
            <Route path="/ok" element={<OkPage />} />
            <Route path="/ko" element={<KoPage />} />
          </Routes>
        </main>
      </div>
    </Router>
  );
}

export default App;