import React from 'react';
import { cn } from "../../lib/utils";

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
  className?: string;
}

export const Modal = ({ isOpen, onClose, children, className }: ModalProps) => {
  if (!isOpen) return null;

  return (
    <div 
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
      onClick={onClose} // Close when clicking outside
    >
      <div 
        className={cn(
          "relative bg-card text-card-foreground rounded-lg shadow-lg max-w-lg mx-auto p-6",
          className
        )}
        onClick={e => e.stopPropagation()} // Prevent click from closing modal
      >
        <button
          onClick={onClose}
          className="absolute top-3 right-3 text-muted-foreground hover:text-foreground"
        >
          &times;
        </button>
        {children}
      </div>
    </div>
  );
}; 