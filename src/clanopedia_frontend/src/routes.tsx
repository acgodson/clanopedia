import { useState } from 'react';
import { Routes, Route } from 'react-router-dom';
import { Toaster } from './components/atoms/toaster';
import { Navbar } from './components/molecules/navbar';
import { useAuth } from './providers/useAuth';
import { Modal } from './components/atoms/modal';
import { LoginModal } from './components/molecules/modals/LoginModal';
import { HomePage } from './pages/HomePage';
import { CollectionPage } from './pages/CollectionPage';
import { GovernancePage } from './pages/GovernancePage';
import { CollectionSettingsPage } from './pages/CollectionSettingsPage';
import { Collections } from './components/organisms/Collections';
import { ProposalsPage } from './pages/ProposalsPage';

export function AppRoutes() {
  const { isAuthenticated, isLoading } = useAuth();
  const [isLoginModalOpen, setIsLoginModalOpen] = useState(false);

  const handleOpenLoginModal = () => {
    setIsLoginModalOpen(true);
  };

  const handleCloseLoginModal = () => {
    setIsLoginModalOpen(false);
  };

  return (
    <div className="min-h-screen flex flex-col">
      <Navbar onLoginClick={handleOpenLoginModal} />
      <main className="flex-1 container mx-auto px-4 py-8 pt-20">
        {isLoading ? (
          <div className="flex items-center justify-center h-[calc(100vh-4rem-64px)]">
            <p>Loading...</p>
          </div>
        ) : (
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/collections" element={<Collections />} />
            <Route path="/collections/:collectionId" element={<CollectionPage />} />
            <Route path="/collections/:collectionId/settings" element={<CollectionSettingsPage />} />
            <Route path="/collections/:collectionId/proposals" element={<ProposalsPage />} />
            <Route path="/governance" element={<GovernancePage />} />
          </Routes>
        )}
      </main>
      <Modal isOpen={isLoginModalOpen} onClose={handleCloseLoginModal}>
        <LoginModal onClose={handleCloseLoginModal} />
      </Modal>
      <Toaster />
    </div>
  );
} 